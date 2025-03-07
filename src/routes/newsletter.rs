#![allow(dead_code)]

use crate::{
    auth::{self, AuthError, Credentials},
    domain::SubscriberEmail,
    email_client::EmailClient,
    routes::error_chain_fmt,
};
use actix_web::{
    http::{header, StatusCode},
    post,
    web::{Data, Json},
    HttpRequest, HttpResponse, Responder, ResponseError,
};
use anyhow::Context;
use base64::Engine;
use secrecy::SecretString;
use serde::Deserialize;
use sqlx::{PgExecutor, PgPool};
use std::fmt::Debug;

#[derive(Deserialize)]
struct BodyData {
    title: String,
    content: Content,
}

#[derive(Deserialize)]
struct Content {
    text: String,
    html: String,
}

#[post("/newsletters")]
#[tracing::instrument(
    name = "Pubish a newsletter issue",
    skip(body, pool, ec, req),
    fields(
        username=tracing::field::Empty,
        user_id=tracing::field::Empty,
    )
)]
pub async fn publish_newsletter(
    body: Json<BodyData>,
    pool: Data<PgPool>,
    ec: Data<EmailClient>,
    req: HttpRequest,
) -> Result<impl Responder, PublishError> {
    let credentials = basic_auth(req.headers()).map_err(PublishError::AuthError)?;
    tracing::Span::current().record("username", tracing::field::display(&credentials.username));
    let user_id = auth::validate_credentials(credentials, pool.as_ref())
        .await
        .map_err(|e| match e {
            AuthError::InvalidCredentials(_) => PublishError::AuthError(e.into()),
            AuthError::UnexpectedError(_) => PublishError::UnexpectedError(e.into()),
        })?;
    tracing::Span::current().record("user_id", tracing::field::display(&user_id));
    let subscribers = get_confirmed_subscribers(&**pool).await?;
    for s in subscribers {
        match s {
            Ok(s) => ec
                .send_email(
                    &s.email,
                    &body.title,
                    &body.content.html,
                    &body.content.text,
                )
                .await
                .with_context(|| format!("Failed to send newsletter issue to {}", s.email))?,
            Err(e) => tracing::warn!(
                error.cause_chain = ?e,
                "Skipping a confirmed subscriber. Their stored contact details are invalid",
            ),
        }
    }
    Ok(HttpResponse::Ok())
}

fn basic_auth(headers: &header::HeaderMap) -> Result<Credentials, anyhow::Error> {
    let decoded_cred = {
        let raw = headers
            .get("Authorization")
            .context("The 'Authorization' header was missing")?
            .to_str()
            .context("The 'Authorization' header was not a valid UTF8 string.")?
            .strip_prefix("Basic ")
            .context("The authorization scheme was not 'Basic'.")?;

        let bytes = base64::engine::general_purpose::STANDARD
            .decode(raw)
            .context("Failed to base64-decode 'Basic' credentials.")?;

        String::from_utf8(bytes).context("The decoded credential string is not valid UTF8.")?
    };

    let mut cred = decoded_cred.splitn(2, ':');

    let username = cred
        .next()
        .context("A username must be provided in 'Basic' auth.")?
        .to_string();
    let password = cred
        .next()
        .context("A password must be provided in 'Basic' auth.")?
        .to_string();

    Ok(Credentials {
        username,
        password: SecretString::from(password),
    })
}

struct ConfirmedSubscriber {
    email: SubscriberEmail,
}

#[tracing::instrument(name = "Get confirmed subscribers", skip(executor))]
async fn get_confirmed_subscribers(
    executor: impl '_ + PgExecutor<'_>,
) -> Result<Vec<Result<ConfirmedSubscriber, anyhow::Error>>, anyhow::Error> {
    let rows = sqlx::query!(
        r#"
        SELECT email FROM subscriptions
        WHERE status = 'confirmed'
        "#
    )
    .fetch_all(executor)
    .await?;

    let confirmed_subscribers = rows
        .into_iter()
        .map(|r| match SubscriberEmail::parse(r.email) {
            Ok(email) => Ok(ConfirmedSubscriber { email }),
            Err(e) => Err(anyhow::anyhow!(e)),
        })
        .collect();
    Ok(confirmed_subscribers)
}

#[derive(thiserror::Error)]
pub enum PublishError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl Debug for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for PublishError {
    fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {
        match self {
            PublishError::UnexpectedError(_) => {
                HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR)
            }
            PublishError::AuthError(_) => {
                let mut resp = HttpResponse::new(StatusCode::UNAUTHORIZED);
                let hv = header::HeaderValue::from_str(r#"Basic realm="publish""#).unwrap();
                resp.headers_mut().insert(header::WWW_AUTHENTICATE, hv);
                resp
            }
        }
    }
}
