use crate::{
    domain::{NewSubscriber, SubscriberEmail, SubscriberName},
    email_client::EmailClient,
    startup::AppBaseUrl,
};
use actix_web::{
    post,
    web::{Data, Form},
    HttpResponse, Responder,
};
use chrono::Utc;
use rand::{distr::Alphanumeric, Rng};
use serde::Deserialize;
use sqlx::{Acquire, PgPool};
use std::iter;
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
) -> impl Responder {
    let ns = match form.0.try_into() {
        Ok(ns) => ns,
        Err(_) => return HttpResponse::BadRequest(),
    };
    let mut transaction = match db_pool.begin().await {
        Ok(t) => t,
        Err(_) => return HttpResponse::InternalServerError(),
    };
    let subscriber_id = match insert_subscriber(&ns, &mut transaction).await {
        Ok(id) => id,
        Err(_) => return HttpResponse::InternalServerError(),
    };
    let subscription_token = generate_subscription_token();
    if store_token(&mut transaction, subscriber_id, &subscription_token)
        .await
        .is_err()
    {
        return HttpResponse::InternalServerError();
    }
    if send_confirmation_email(&email_client, &ns, &base_url.0, &subscription_token)
        .await
        .is_err()
    {
        return HttpResponse::InternalServerError();
    }
    if transaction.commit().await.is_err() {
        return HttpResponse::InternalServerError();
    }
    HttpResponse::Ok()
}

#[tracing::instrument(
    name = "Storing the subscription token for the new subscriber in the database",
    skip(executor, subscription_token)
)]
async fn store_token(
    executor: impl Acquire<'_, Database = sqlx::Postgres>,
    subscriber_id: Uuid,
    subscription_token: &str,
) -> Result<(), sqlx::Error> {
    let executor = &mut *(executor.acquire().await?);

    sqlx::query!(
        r#"INSERT INTO subscription_tokens (subscriber_id, token)
    VALUES ($1, $2)"#,
        subscriber_id,
        subscription_token
    )
    .execute(executor)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(())
}

/// Generate a random 25-characters-long case-sensitive subscription token.
fn generate_subscription_token() -> String {
    iter::repeat_with(|| rand::rng().sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}

#[tracing::instrument(
    name = "Sending a confirmation email to a new subscriber",
    skip(ec, ns, base_url, token)
)]
async fn send_confirmation_email(
    ec: &EmailClient,
    ns: &NewSubscriber,
    base_url: &str,
    token: &str,
) -> Result<(), reqwest::Error> {
    let confirmation_link = format!("{}/subscriptions/confirm?token={}", base_url, token);

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

#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(ns, executor)
)]
async fn insert_subscriber(
    ns: &NewSubscriber,
    executor: impl Acquire<'_, Database = sqlx::Postgres>,
) -> Result<Uuid, sqlx::Error> {
    let executor = &mut *(executor.acquire().await?);

    let id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at, status)
        VALUES ($1, $2, $3, $4, 'pending_confirmation')
        "#,
        id,
        ns.email.as_ref(),
        ns.name.as_ref(),
        Utc::now(),
    )
    .execute(executor)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(id)
}
