use crate::{
    auth::UserId,
    idempotency::{self, IdempotencyKey, NextAction},
    utils,
};
use actix_web::{post, web, Responder};
use actix_web_flash_messages::FlashMessage;
use anyhow::Context;
use serde::Deserialize;
use sqlx::{PgExecutor, PgPool};
use uuid::Uuid;

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
    skip_all,
    fields( user_id = %*user_id)
)]
pub async fn publish_newsletter(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
) -> actix_web::Result<impl Responder> {
    let user_id = user_id.into_inner();
    let FormData {
        title,
        text,
        html,
        idempotency_key,
    } = form.0;
    let idempotency_key: IdempotencyKey = idempotency_key.try_into().map_err(utils::e400)?;

    let mut txn = match idempotency::try_processing(*user_id, &idempotency_key, &pool)
        .await
        .map_err(utils::e500)?
    {
        NextAction::StartProcessing(t) => t,
        NextAction::ReturnSavedResponse(r) => {
            success_message().send();
            return Ok(r);
        }
    };

    let issue_id = insert_newsletter_issue(&title, &text, &html, txn.as_mut())
        .await
        .context("Failed to store newsletter issue details")
        .map_err(utils::e500)?;

    enqueue_delivery_tasks(issue_id, txn.as_mut())
        .await
        .context("Failed to enqueue delivery tasks")
        .map_err(utils::e500)?;

    success_message().send();
    let resp = {
        let resp = utils::see_other("/admin/newsletters");
        idempotency::save_response(resp, *user_id, &idempotency_key, txn)
            .await
            .map_err(utils::e500)?
    };
    Ok(resp)
}

fn success_message() -> FlashMessage {
    FlashMessage::info("The newsletter issue has been accepted - emails will go out shortly.")
}

#[tracing::instrument(skip_all)]
async fn insert_newsletter_issue(
    title: &str,
    text_content: &str,
    html_content: &str,
    exec: impl PgExecutor<'_>,
) -> anyhow::Result<Uuid> {
    let issue_id = Uuid::new_v4();
    sqlx::query!(
        r#"
    INSERT INTO newsletter_issues (
        id,
        title,
        text_content,
        html_content,
        published_at
    )
    VALUES ($1, $2, $3, $4, now())
    "#,
        issue_id,
        title,
        text_content,
        html_content
    )
    .execute(exec)
    .await?;
    Ok(issue_id)
}

#[tracing::instrument(skip_all)]
async fn enqueue_delivery_tasks(issue_id: Uuid, exec: impl PgExecutor<'_>) -> anyhow::Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO issue_delivery_queue (
            newsletter_issue_id,
            subscriber_email
        )
        SELECT $1, email
        FROM subscriptions
        WHERE status = 'confirmed'
    "#,
        issue_id
    )
    .execute(exec)
    .await?;
    Ok(())
}
