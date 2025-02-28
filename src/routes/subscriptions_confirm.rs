use actix_web::{
    get,
    web::{Data, Query},
    HttpResponse, Responder,
};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Deserialize)]
struct Parameters {
    token: String,
}

#[get("/subscriptions/confirm")]
#[tracing::instrument(name = "Confirming a pending subscriber", skip(db_pool, parameters))]
pub async fn confirm(db_pool: Data<PgPool>, parameters: Query<Parameters>) -> impl Responder {
    let id = match get_subscriber_id_from_token(&db_pool, &parameters.token).await {
        Ok(id) => match id {
            Some(id) => id,
            None => return HttpResponse::Unauthorized(),
        },
        Err(_) => return HttpResponse::InternalServerError(),
    };

    if confirm_subscriber(&db_pool, id).await.is_err() {
        return HttpResponse::InternalServerError();
    }

    HttpResponse::Ok()
}

#[tracing::instrument(name = "Mark subscriber as confirmed", skip(subscriber_id, db_pool))]
async fn confirm_subscriber(db_pool: &PgPool, subscriber_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE subscriptions SET status = 'confirmed' WHERE id = $1",
        subscriber_id
    )
    .execute(db_pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;

    Ok(())
}

#[tracing::instrument(name = "Get subscriber_id from token", skip(db_pool, token))]
async fn get_subscriber_id_from_token(
    db_pool: &PgPool,
    token: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    let id = sqlx::query!(
        "SELECT subscriber_id FROM subscription_tokens WHERE token = $1",
        token
    )
    .fetch_optional(db_pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?
    .map(|r| r.subscriber_id);

    Ok(id)
}
