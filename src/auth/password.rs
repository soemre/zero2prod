use crate::{domain::ValidPassword, routes::error_chain_fmt, telemetry};
use anyhow::Context;
use argon2::{
    password_hash::SaltString, Algorithm, Argon2, Params, PasswordHash, PasswordHasher,
    PasswordVerifier, Version,
};
use secrecy::{ExposeSecret, SecretString};
use sqlx::PgExecutor;
use std::fmt::Debug;
use uuid::Uuid;

#[derive(thiserror::Error)]
pub enum AuthError {
    #[error("Invalid credentials.")]
    InvalidCredentials(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl Debug for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

pub struct Credentials {
    pub username: String,
    pub password: SecretString,
}

#[tracing::instrument(name = "Validate credentials", skip(c, executor))]
pub async fn validate_credentials(
    c: Credentials,
    executor: impl '_ + PgExecutor<'_>,
) -> Result<Uuid, AuthError> {
    const DUMMY_PASSWORD_PHC: &str = "$argon2id$v=19$m=15000,t=2,p=1$gZiV/M1gPc22ElAH/Jh1Hw$CWOrkoo7oJBQ/iyh7uJ0LO2aLEfrHwTWllSAxT0zRno";

    let (id, expected_password_hash) = get_stored_credentials(&c.username, executor).await?.map_or(
        (None, SecretString::from(DUMMY_PASSWORD_PHC)),
        |(id, hash)| (Some(id), hash),
    );

    telemetry::spawn_blocking_with_tracing(|| {
        verify_password_hash(expected_password_hash, c.password)
    })
    .await
    .context("Failed to spawn blocking task.")??;

    id.ok_or_else(|| AuthError::InvalidCredentials(anyhow::anyhow!("Unknown username.")))
}

#[tracing::instrument(name = "Verify password hash", skip(expected, candidate))]
fn verify_password_hash(expected: SecretString, candidate: SecretString) -> Result<(), AuthError> {
    let expected = PasswordHash::new(expected.expose_secret())
        .context("Failed to parse hash in PHC string format.")?;

    Argon2::default()
        .verify_password(candidate.expose_secret().as_bytes(), &expected)
        .context("Invalid password.")
        .map_err(AuthError::InvalidCredentials)
}

#[tracing::instrument(name = "Get stored credentials", skip(username, executor))]
async fn get_stored_credentials(
    username: &str,
    executor: impl '_ + PgExecutor<'_>,
) -> Result<Option<(Uuid, SecretString)>, anyhow::Error> {
    let r = sqlx::query!(
        r#"
            SELECT id, password_hash 
            FROM users
            WHERE username = $1 
        "#,
        username
    )
    .fetch_optional(executor)
    .await
    .context("Failed to perform a query to validate auth credentials.")?
    .map(|r| (r.id, SecretString::from(r.password_hash)));

    Ok(r)
}

#[tracing::instrument(name = "Change password", skip(password, executor))]
pub async fn change_password(
    user_id: Uuid,
    password: ValidPassword,
    executor: impl '_ + PgExecutor<'_>,
) -> anyhow::Result<()> {
    let password_hash = telemetry::spawn_blocking_with_tracing(|| compute_password_hash(password))
        .await?
        .context("Failed to hash password")?;

    sqlx::query!(
        r#"
    UPDATE users
    SET password_hash = $1
    WHERE id = $2
    "#,
        password_hash.expose_secret(),
        user_id
    )
    .execute(executor)
    .await
    .context("Failed to change user's password in the database.")?;

    Ok(())
}

fn compute_password_hash(password: ValidPassword) -> anyhow::Result<SecretString> {
    let salt = SaltString::generate(rand::thread_rng());

    let password_hash = Argon2::new(
        Algorithm::Argon2id,
        Version::V0x13,
        Params::new(15000, 2, 1, None).unwrap(),
    )
    .hash_password(password.inner().expose_secret().as_bytes(), &salt)?
    .to_string();

    Ok(SecretString::from(password_hash))
}
