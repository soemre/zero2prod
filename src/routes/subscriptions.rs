use crate::{
    app::AppBaseUrl,
    domain::{NewSubscriber, SubscriberEmail, SubscriberName, SubscriptionToken},
    email_client::EmailClient,
};
use actix_web::{
    http::StatusCode,
    post,
    web::{Data, Form},
    HttpResponse, Responder, ResponseError,
};
use anyhow::Context;
use chrono::Utc;
use serde::Deserialize;
use sqlx::{PgExecutor, PgPool};
use std::{
    error::Error,
    fmt::{Debug, Display},
};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct SubscriptionForm {
    name: String,
    email: String,
}

#[post("/subscriptions")]
#[tracing::instrument(
    name ="Adding a new subscriber",
    skip(form, db_pool, email_client, base_url),
    fields(
        subscriber_email = %form.email,
        subscriber_name = %form.name,
    )
)]
pub async fn subscribe(
    form: Form<SubscriptionForm>,
    db_pool: Data<PgPool>,
    email_client: Data<EmailClient>,
    base_url: Data<AppBaseUrl>,
) -> Result<impl Responder, SubscribeError> {
    let ns = form.0.try_into().map_err(SubscribeError::ValidationError)?;
    let mut txn = db_pool
        .begin()
        .await
        .context("Failed to acquire a Postgres connection from the pool")?;
    let subscriber_id = insert_subscriber(&ns, txn.as_mut())
        .await
        .context("Failed to insert new subscriber in the database.")?;
    let token = SubscriptionToken::generate();
    store_token(txn.as_mut(), subscriber_id, &token)
        .await
        .context("Failed to store the confirmation token for a new subscriber.")?;
    send_confirmation_email(&email_client, &ns, &base_url.0, &token)
        .await
        .context("Failed to send a confirmation email.")?;
    txn.commit()
        .await
        .context("Failed to commit SQL transaction to store a new subscriber.")?;
    Ok(HttpResponse::Ok())
}

/// Stores the given token. If the user already has a token assigned, overwrites it.
#[tracing::instrument(
    name = "Storing the subscription token for the new subscriber in the database",
    skip(executor, token)
)]
async fn store_token(
    executor: impl '_ + PgExecutor<'_>,
    subscriber_id: Uuid,
    token: &SubscriptionToken,
) -> Result<(), StoreTokenError> {
    sqlx::query!(
        r#"
        INSERT INTO subscription_tokens (subscriber_id, token)
        VALUES ($1, $2)
        ON CONFLICT (subscriber_id)
        DO UPDATE
        SET token = $2
        "#,
        subscriber_id,
        token.as_ref()
    )
    .execute(executor)
    .await?;
    Ok(())
}

#[tracing::instrument(
    name = "Sending a confirmation email to a new subscriber",
    skip(ec, ns, base_url, token)
)]
async fn send_confirmation_email(
    ec: &EmailClient,
    ns: &NewSubscriber,
    base_url: &str,
    token: &SubscriptionToken,
) -> Result<(), reqwest::Error> {
    let confirmation_link = format!(
        "{}/subscriptions/confirm?token={}",
        base_url,
        token.as_ref()
    );

    let html_body = format!(
        "Welcome to our newsletter!<br />\
                Click <a href=\"{}\">here</a> to confirm your subscription.",
        confirmation_link
    );

    let text_body = format!(
        "Welcome to our newsletter!\nVisit {} to confirm your subscription.",
        confirmation_link
    );

    ec.send_email(&ns.email, "Welcome!", &html_body, &text_body)
        .await
}

impl TryFrom<SubscriptionForm> for NewSubscriber {
    type Error = String;

    fn try_from(form: SubscriptionForm) -> Result<Self, Self::Error> {
        let name = SubscriberName::parse(form.name)?;
        let email = SubscriberEmail::parse(form.email)?;

        Ok(NewSubscriber { name, email })
    }
}

/// Inserts a new user with the given information if the user doesn't already exist.
/// Returns the `Uuid` of the user with the given email.
#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(ns, executor)
)]
async fn insert_subscriber(
    ns: &NewSubscriber,
    executor: impl '_ + PgExecutor<'_>,
) -> Result<Uuid, sqlx::Error> {
    let id = {
        let new_id = Uuid::new_v4();

        sqlx::query!(
            r#"
            INSERT INTO subscriptions (id, email, name, subscribed_at, status)
            VALUES ($1, $2, $3, $4, 'pending_confirmation')
            ON CONFLICT (email) DO UPDATE
            SET email = EXCLUDED.email
            RETURNING id
            "#,
            new_id,
            ns.email.as_ref(),
            ns.name.as_ref(),
            Utc::now(),
        )
        .fetch_one(executor)
        .await?
        .id
    };
    Ok(id)
}

pub fn error_chain_fmt(e: &dyn Error, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    writeln!(f, "{}\n", e)?;
    let mut e = Some(e);
    let e_iter = std::iter::from_fn(move || {
        e = e?.source();
        e
    });
    for e in e_iter {
        writeln!(f, "Caused by:\n\t{}", e)?;
    }
    Ok(())
}

pub struct StoreTokenError(sqlx::Error);

impl Debug for StoreTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl Display for StoreTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "A database error was encountered while trying to store a subscription token."
        )
    }
}

impl Error for StoreTokenError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.0)
    }
}

impl From<sqlx::Error> for StoreTokenError {
    fn from(value: sqlx::Error) -> Self {
        StoreTokenError(value)
    }
}

#[derive(thiserror::Error)]
pub enum SubscribeError {
    #[error("{0}")]
    ValidationError(String),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl ResponseError for SubscribeError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match self {
            Self::ValidationError(_) => StatusCode::BAD_REQUEST,
            Self::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl Debug for SubscribeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}
