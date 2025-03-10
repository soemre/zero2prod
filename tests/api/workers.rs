use crate::helpers::TestApp;
use chrono::{Local, TimeDelta};
use uuid::Uuid;
use zero2prod::workers::expiration;

#[tokio::test]
async fn experied_idempotency_keys_are_deleted() {
    // Arrange
    let app = TestApp::spawn().await;

    app.insert_a_new_idempotency_key(48).await;

    // Act
    expiration::delete_expired(&app.db_pool).await.unwrap();

    // Assert
    let rows_found = app.count_stored_idempotency_keys().await;

    assert_eq!(0, rows_found);
}

#[tokio::test]
async fn valid_idempotency_keys_are_not_deleted() {
    // Arrange
    let app = TestApp::spawn().await;

    app.insert_a_new_idempotency_key(1).await;

    // Act
    expiration::delete_expired(&app.db_pool).await.unwrap();

    // Assert
    let rows_found = app.count_stored_idempotency_keys().await;

    assert_eq!(1, rows_found);
}

impl TestApp {
    async fn insert_a_new_idempotency_key(&self, sub_hours: i64) {
        let created_at = Local::now()
            .checked_sub_signed(TimeDelta::hours(sub_hours))
            .unwrap();

        let rows_affected = sqlx::query!(
            r#"
            INSERT INTO idempotency (
                user_id,
                idempotency_key,
                created_at
            )
            VALUES ($1, $2, $3)
            ON CONFLICT DO NOTHING
            "#,
            self.test_user.id,
            Uuid::new_v4().to_string(),
            created_at
        )
        .execute(&self.db_pool)
        .await
        .expect("Failed to insert an idempotency key into the database")
        .rows_affected();

        assert_eq!(1, rows_affected);
    }

    async fn count_stored_idempotency_keys(&self) -> i64 {
        sqlx::query!(r#"SELECT COUNT(*) FROM idempotency"#)
            .fetch_one(&self.db_pool)
            .await
            .expect("Failed to insert an idempotency key into the DB.")
            .count
            .unwrap()
    }
}
