use crate::{
    auth::UserId,
    domain::SubscriberEmail,
    email_client::EmailClient,
    idempotency::{self, IdempotencyKey},
    utils,
};
use actix_web::{post, web, Responder};
use actix_web_flash_messages::FlashMessage;
use anyhow::Context;
use serde::Deserialize;
use sqlx::{PgExecutor, PgPool};
use std::fmt::Debug;

#[derive(Deserialize)]
struct FormData {
    title: String,
    text: String,
    html: String,
    idempotency_key: String,
}

#[post("/newsletters")]
#[tracing::instrument(
    name = "Pubish a newsletter issue",
    skip(form, pool, ec, user_id),
    fields( user_id = %*user_id)
)]
pub async fn publish_newsletter(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
    ec: web::Data<EmailClient>,
    user_id: web::ReqData<UserId>,
) -> actix_web::Result<impl Responder> {
    let FormData {
        title,
        text,
        html,
        idempotency_key,
    } = form.0;

    let idempotency_key: IdempotencyKey = idempotency_key.try_into().map_err(utils::e400)?;
    if let Some(r) = idempotency::get_saved_response(**user_id, &idempotency_key, pool.as_ref())
        .await
        .map_err(utils::e500)?
    {
        FlashMessage::info("All done! The newsletter has been published.").send();
        return Ok(r);
    }

    let subscribers = get_confirmed_subscribers(pool.as_ref())
        .await
        .map_err(utils::e500)?;

    for s in subscribers {
        match s {
            Ok(s) => ec
                .send_email(&s.email, &title, &html, &text)
                .await
                .with_context(|| format!("Failed to send newsletter issue to {}", s.email))
                .map_err(utils::e500)?,
            Err(e) => tracing::warn!(
                error.cause_chain = ?e,
                "Skipping a confirmed subscriber. Their stored contact details are invalid",
            ),
        }
    }

    FlashMessage::info("All done! The newsletter has been published.").send();
    let resp = {
        let resp = utils::see_other("/admin/newsletters");
        idempotency::save_response(resp, **user_id, &idempotency_key, pool.as_ref())
            .await
            .map_err(utils::e500)?
    };
    Ok(resp)
}

struct ConfirmedSubscriber {
    email: SubscriberEmail,
}

#[tracing::instrument(name = "Get confirmed subscribers", skip(executor))]
async fn get_confirmed_subscribers(
    executor: impl '_ + PgExecutor<'_>,
) -> anyhow::Result<Vec<anyhow::Result<ConfirmedSubscriber>>> {
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
