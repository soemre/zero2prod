#![allow(dead_code)]

use crate::{domain::SubscriberEmail, email_client::EmailClient, routes::error_chain_fmt};
use actix_web::{
    http::StatusCode,
    post,
    web::{Data, Json},
    HttpResponse, Responder, ResponseError,
};
use anyhow::Context;
use serde::Deserialize;
use sqlx::{Acquire, PgPool};
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
pub async fn publish_newsletter(
    body: Json<BodyData>,
    pool: Data<PgPool>,
    ec: Data<EmailClient>,
) -> Result<impl Responder, PublishError> {
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

struct ConfirmedSubscriber {
    email: SubscriberEmail,
}

#[tracing::instrument(name = "Get confirmed subscribers", skip(executor))]
async fn get_confirmed_subscribers(
    executor: impl Acquire<'_, Database = sqlx::Postgres>,
) -> Result<Vec<Result<ConfirmedSubscriber, anyhow::Error>>, anyhow::Error> {
    let executor = &mut *(executor.acquire().await?);
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
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl Debug for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for PublishError {
    fn status_code(&self) -> StatusCode {
        match self {
            PublishError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            // _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
