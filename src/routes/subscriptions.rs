use actix_web::{
    post,
    web::{Data, Form},
    HttpResponse, Responder,
};
use chrono::Utc;
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::{NewSubscriber, SubscriberName};

#[derive(Deserialize)]
pub struct SubscriptionForm {
    name: String,
    email: String,
}

#[post("/subscriptions")]
#[tracing::instrument(
    name ="Adding a new subscriber",
    skip(form, db_pool),
    fields(
        subscriber_email = %form.email,
        subscriber_name = %form.name,
    )
)]
pub async fn subscribe(form: Form<SubscriptionForm>, db_pool: Data<PgPool>) -> impl Responder {
    let name = match SubscriberName::parse(&form.name) {
        Ok(name) => name,
        Err(_) => return HttpResponse::BadRequest(),
    };
    let ns = NewSubscriber {
        name,
        email: form.0.email,
    };
    match insert_subscriber(&ns, &db_pool).await {
        Ok(_) => HttpResponse::Ok(),
        Err(_) => HttpResponse::InternalServerError(),
    }
}

#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(ns, db_pool)
)]
async fn insert_subscriber(ns: &NewSubscriber, db_pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at)
        VALUES ($1, $2, $3, $4)
        "#,
        Uuid::new_v4(),
        ns.email,
        ns.name.as_ref(),
        Utc::now(),
    )
    .execute(db_pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(())
}
