use crate::config::Settings;
use sqlx::{PgExecutor, PgPool};
use std::time::Duration;

pub struct Worker {
    pool: PgPool,
}

impl Worker {
    pub fn builder(config: &Settings) -> Self {
        let pool = config.database.get_db_pool();
        Self { pool }
    }

    pub async fn finish(self) -> anyhow::Result<()> {
        loop {
            delete_expired(&self.pool).await?;
            tokio::time::sleep(Duration::from_secs(120)).await;
        }
    }
}

pub async fn delete_expired(exec: impl PgExecutor<'_>) -> anyhow::Result<()> {
    sqlx::query!(
        r#"
    DELETE FROM idempotency
    WHERE created_at < now() - interval '1 day'
    "#
    )
    .execute(exec)
    .await?;
    Ok(())
}
