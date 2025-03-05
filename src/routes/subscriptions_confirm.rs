use crate::{domain::SubscriptionToken, routes::error_chain_fmt};
use actix_web::{
    get,
    http::StatusCode,
    web::{Data, Query},
    HttpResponse, Responder,
};
use anyhow::Context;
use serde::Deserialize;
use sqlx::{PgExecutor, PgPool};
use std::fmt::Debug;
use uuid::Uuid;

#[derive(Deserialize)]
struct Parameters {
    token: String,
}

#[get("/subscriptions/confirm")]
#[tracing::instrument(name = "Confirming a pending subscriber", skip(db_pool, parameters))]
pub async fn confirm(
    db_pool: Data<PgPool>,
    parameters: Query<Parameters>,
) -> Result<impl Responder, ConfirmSubscriberError> {
    let token = SubscriptionToken::parse(parameters.token.clone())
        .map_err(ConfirmSubscriberError::InvalidTokenFormat)?;
    let mut txn = db_pool
        .begin()
        .await
        .context("Failed to acquire a Postgres connection from the pool")?;
    let id = consume_subscriber_id_from_token(txn.as_mut(), &token)
        .await
        .context("Failed to attempt token consumption for the specified user.")?
        .ok_or(ConfirmSubscriberError::UnknownToken)?;
    confirm_subscriber(txn.as_mut(), id)
        .await
        .context("Failed to confirm the user.")?;
    txn.commit()
        .await
        .context("Failed to commit SQL transaction to store a new subscriber.")?;

    Ok(HttpResponse::Ok())
}

#[tracing::instrument(name = "Mark subscriber as confirmed", skip(subscriber_id, executor))]
async fn confirm_subscriber(
    executor: impl '_ + PgExecutor<'_>,
    subscriber_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE subscriptions SET status = 'confirmed' WHERE id = $1",
        subscriber_id
    )
    .execute(executor)
    .await?;

    Ok(())
}

/// Returns the `subscriber_id` for the given token by removing the corresponding
/// token entry.
#[tracing::instrument(name = "Consume subscriber_id from token", skip(executor, token))]
async fn consume_subscriber_id_from_token(
    executor: impl '_ + PgExecutor<'_>,
    token: &SubscriptionToken,
) -> Result<Option<Uuid>, sqlx::Error> {
    let id = sqlx::query!(
        "DELETE FROM subscription_tokens WHERE token = $1 RETURNING subscriber_id",
        token.as_ref()
    )
    .fetch_optional(executor)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?
    .map(|r| r.subscriber_id);

    Ok(id)
}

#[derive(thiserror::Error)]
pub enum ConfirmSubscriberError {
    #[error("{0}")]
    InvalidTokenFormat(String),
    #[error("No record has been found for the given token.")]
    UnknownToken,
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl Debug for ConfirmSubscriberError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl actix_web::ResponseError for ConfirmSubscriberError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match self {
            Self::InvalidTokenFormat(_) => StatusCode::BAD_REQUEST,
            Self::UnknownToken => StatusCode::UNAUTHORIZED,
            Self::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
