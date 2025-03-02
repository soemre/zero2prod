use crate::{
    domain::{NewSubscriber, SubscriberEmail, SubscriberName, SubscriptionToken},
    email_client::EmailClient,
    startup::AppBaseUrl,
};
use actix_web::{
    post,
    web::{Data, Form},
    HttpResponse, Responder,
};
use chrono::Utc;
use serde::Deserialize;
use sqlx::{Acquire, PgPool};
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
    let mut txn = match db_pool.begin().await {
        Ok(t) => t,
        Err(_) => return HttpResponse::InternalServerError(),
    };
    let subscriber_id = match insert_subscriber(&ns, &mut txn).await {
        Ok(id) => id,
        Err(_) => return HttpResponse::InternalServerError(),
    };
    let token = SubscriptionToken::generate();
    if store_token(&mut txn, subscriber_id, &token).await.is_err() {
        return HttpResponse::InternalServerError();
    }
    if send_confirmation_email(&email_client, &ns, &base_url.0, &token)
        .await
        .is_err()
    {
        return HttpResponse::InternalServerError();
    }
    if txn.commit().await.is_err() {
        return HttpResponse::InternalServerError();
    }
    HttpResponse::Ok()
}

/// Stores the given token. If the user already has a token assigned, overwrites it.
#[tracing::instrument(
    name = "Storing the subscription token for the new subscriber in the database",
    skip(executor, token)
)]
async fn store_token(
    executor: impl Acquire<'_, Database = sqlx::Postgres>,
    subscriber_id: Uuid,
    token: &SubscriptionToken,
) -> Result<(), sqlx::Error> {
    let executor = &mut *(executor.acquire().await?);

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
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
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
    executor: impl Acquire<'_, Database = sqlx::Postgres>,
) -> Result<Uuid, sqlx::Error> {
    let executor = &mut *(executor.acquire().await?);

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
        .await
        .map_err(|e| {
            tracing::error!("Failed to execute query: {:?}", e);
            e
        })?
        .id
    };
    Ok(id)
}
