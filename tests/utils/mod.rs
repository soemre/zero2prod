use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::net::TcpListener;
use uuid::Uuid;

use zero2prod::{
    config::{self, DatabaseSettings},
    startup,
};

const DB_CONNECTION_FAIL: &str = "Failed to connect to Postgres";

pub struct TestApp {
    pub addr: String,
    pub db_pool: PgPool,
}

impl TestApp {
    /// Runs the app in the background at a random port
    /// and returns the bound address in "http://addr:port" format.
    pub async fn spawn() -> TestApp {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("Failed to bind address.");
        let addr = listener.local_addr().unwrap();
        let addr = format!("http://{}:{}", addr.ip(), addr.port());

        let mut config = config::get().expect("Failed to read configuration");
        config.database.name = Uuid::new_v4().to_string();

        let db_pool = spawn_db(&config.database).await;

        let server = startup::run(listener, db_pool.clone()).expect("Failed to run the server.");
        tokio::spawn(server);

        TestApp { db_pool, addr }
    }
}

async fn spawn_db(config: &DatabaseSettings) -> PgPool {
    // Create Database
    let maintenance_settings = DatabaseSettings {
        name: "postgres".to_string(),
        username: "postgres".to_string(),
        password: "password".to_string(),
        ..config.clone()
    };

    PgConnection::connect(&maintenance_settings.url())
        .await
        .expect(DB_CONNECTION_FAIL)
        .execute(format!(r#"CREATE DATABASE "{}";"#, config.name).as_str())
        .await
        .expect("Failed to create database");

    // Migrate Database
    let db_pool = PgPool::connect(&config.url())
        .await
        .expect(DB_CONNECTION_FAIL);

    sqlx::migrate!("./migrations")
        .run(&db_pool)
        .await
        .expect("Failed to migrate the database");

    db_pool
}
