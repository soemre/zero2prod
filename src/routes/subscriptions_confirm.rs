use crate::domain::SubscriptionToken;
use actix_web::{
    get,
    web::{Data, Query},
    HttpResponse, Responder,
};
use serde::Deserialize;
use sqlx::{Acquire, PgPool};
use uuid::Uuid;

#[derive(Deserialize)]
struct Parameters {
    token: String,
}

#[get("/subscriptions/confirm")]
#[tracing::instrument(name = "Confirming a pending subscriber", skip(db_pool, parameters))]
pub async fn confirm(db_pool: Data<PgPool>, parameters: Query<Parameters>) -> impl Responder {
    let token = match SubscriptionToken::parse(parameters.token.clone()) {
        Ok(t) => t,
        Err(_) => return HttpResponse::BadRequest(),
    };
    let mut txn = match db_pool.begin().await {
        Ok(t) => t,
        Err(_) => return HttpResponse::InternalServerError(),
    };
    let id = match get_subscriber_id_from_token(&mut txn, &token).await {
        Ok(id) => match id {
            Some(id) => id,
            None => return HttpResponse::Unauthorized(),
        },
        Err(_) => return HttpResponse::InternalServerError(),
    };

    if confirm_subscriber(&mut txn, id).await.is_err() {
        return HttpResponse::InternalServerError();
    }
    if txn.commit().await.is_err() {
        return HttpResponse::InternalServerError();
    }

    HttpResponse::Ok()
}

#[tracing::instrument(name = "Mark subscriber as confirmed", skip(subscriber_id, executor))]
async fn confirm_subscriber(
    executor: impl Acquire<'_, Database = sqlx::Postgres>,
    subscriber_id: Uuid,
) -> Result<(), sqlx::Error> {
    let executor = &mut *(executor.acquire().await?);

    sqlx::query!(
        "UPDATE subscriptions SET status = 'confirmed' WHERE id = $1",
        subscriber_id
    )
    .execute(executor)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;

    Ok(())
}

#[tracing::instrument(name = "Get subscriber_id from token", skip(executor, token))]
async fn get_subscriber_id_from_token(
    executor: impl Acquire<'_, Database = sqlx::Postgres>,
    token: &SubscriptionToken,
) -> Result<Option<Uuid>, sqlx::Error> {
    let executor = &mut *(executor.acquire().await?);

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
