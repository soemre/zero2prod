use crate::{config::Settings, domain::SubscriberEmail, email_client::EmailClient};
use sqlx::{PgExecutor, PgPool};
use std::time::Duration;
use tracing::Span;
use uuid::Uuid;

pub struct Worker {
    pool: PgPool,
    email_client: EmailClient,
}
impl Worker {
    pub fn builder(config: &Settings) -> Self {
        let pool = config.database.get_db_pool();
        let email_client = config.email_client.client();
        Self { pool, email_client }
    }

    pub async fn finish(self) -> anyhow::Result<()> {
        loop {
            match try_execute_task(&self.pool, &self.email_client).await {
                Err(_) => tokio::time::sleep(Duration::from_secs(1)).await,
                Ok(ExecutionOutcome::EmptyQueue) => {
                    tokio::time::sleep(Duration::from_secs(10)).await
                }
                Ok(ExecutionOutcome::TaskCompleted) => (),
            }
        }
    }
}

#[must_use]
pub enum ExecutionOutcome {
    TaskCompleted,
    EmptyQueue,
}

#[tracing::instrument(
    skip_all,
    fields(
        newsletter_issue_id=tracing::field::Empty,
        subscriber_email=tracing::field::Empty,
    )
    err
)]
pub async fn try_execute_task(
    pool: &PgPool,
    email_client: &EmailClient,
) -> anyhow::Result<ExecutionOutcome> {
    let mut txn = pool.begin().await?;
    let (issue_id, email) = match dequeue_task(txn.as_mut()).await? {
        Some(v) => v,
        None => return Ok(ExecutionOutcome::EmptyQueue),
    };
    Span::current()
        .record("newsletter_issue_id", tracing::field::display(issue_id))
        .record("subscriber_email", tracing::field::display(&email));
    match SubscriberEmail::parse(email.clone()) {
        Err(e) => {
            tracing::error!(
                error.cause_chain = ?e,
                error.message = %e,
                "Skipping a confirmed subscriber. Their stored contact details are invalid",
            );
        }
        Ok(email) => {
            let issue = get_issue(txn.as_mut(), issue_id).await?;
            if let Err(e) = email_client
                .send_email(
                    &email,
                    &issue.title,
                    &issue.html_content,
                    &issue.text_content,
                )
                .await
            {
                tracing::error!(
                    error.cause_chain = ?e,
                    error.message = %e,
                    "Failed to deliver issue to a confirmed subscriber. Skipping.",
                );
            }
        }
    }
    delete_task(txn.as_mut(), issue_id, &email).await?;

    txn.commit().await?;
    Ok(ExecutionOutcome::TaskCompleted)
}

#[tracing::instrument(skip_all)]
async fn dequeue_task(exec: impl PgExecutor<'_>) -> anyhow::Result<Option<(Uuid, String)>> {
    let r = sqlx::query!(
        r#"
    SELECT 
        newsletter_issue_id,
        subscriber_email
    FROM issue_delivery_queue
    FOR UPDATE
    SKIP LOCKED
    LIMIT 1
    "#
    )
    .fetch_optional(exec)
    .await?
    .map(|r| (r.newsletter_issue_id, r.subscriber_email));
    Ok(r)
}

#[tracing::instrument(skip_all)]
async fn delete_task(
    exec: impl PgExecutor<'_>,
    newsletter_issue_id: Uuid,
    email: &str,
) -> anyhow::Result<()> {
    sqlx::query!(
        r#"
    DELETE FROM issue_delivery_queue
    WHERE newsletter_issue_id = $1
        AND subscriber_email = $2
    "#,
        newsletter_issue_id,
        email
    )
    .execute(exec)
    .await?;
    Ok(())
}

struct NewsletterIssue {
    title: String,
    text_content: String,
    html_content: String,
}

#[tracing::instrument(skip_all)]
async fn get_issue(exec: impl PgExecutor<'_>, issue_id: Uuid) -> anyhow::Result<NewsletterIssue> {
    let issue = sqlx::query_as!(
        NewsletterIssue,
        r#"
        SELECT title, text_content, html_content
        FROM newsletter_issues
        WHERE
            id = $1
        "#,
        issue_id,
    )
    .fetch_one(exec)
    .await?;
    Ok(issue)
}
