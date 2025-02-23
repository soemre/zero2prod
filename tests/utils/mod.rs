use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::{env, io, net::TcpListener, sync::LazyLock};
use uuid::Uuid;
use zero2prod::{
    config::{self, DatabaseSettings},
    email_client::EmailClient,
    startup, telemetry,
};

const DB_CONNECTION_FAIL: &str = "Failed to connect to Postgres";

const LOGGER_NAME: &str = "test";
const LOGGER_FILTER_LEVEL: &str = "info";

static TRACING: LazyLock<()> = LazyLock::new(TestApp::init_logging);

#[allow(dead_code)]
pub struct TestApp {
    pub addr: String,
    pub db_pool: PgPool,
}

impl TestApp {
    /// Runs the app in the background at a random port
    /// and returns the bound address in "http://addr:port" format.
    pub async fn spawn() -> TestApp {
        LazyLock::force(&TRACING);

        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("Failed to bind address.");
        let addr = {
            let raw = listener.local_addr().unwrap();
            format!("http://{}:{}", raw.ip(), raw.port())
        };

        let config = {
            let mut raw = config::get().expect("Failed to read configuration");
            raw.database.name = Uuid::new_v4().to_string();
            raw
        };

        let db_pool = TestApp::init_db(&config.database).await;

        let email_client = {
            let sender = config
                .email_client
                .sender()
                .expect("Invalid sender email address.");
            EmailClient::new(config.email_client.base_url, sender)
        };

        let server = startup::run(listener, PgPool::clone(&db_pool), email_client)
            .expect("Failed to run the server.");
        tokio::spawn(server);

        TestApp { db_pool, addr }
    }

    fn init_logging() {
        let subscriber: Box<dyn tracing::subscriber::Subscriber + Send + Sync> =
            if env::var("TEST_LOG").is_ok() {
                Box::new(telemetry::get_subscriber(
                    LOGGER_NAME,
                    LOGGER_FILTER_LEVEL,
                    io::stdout,
                ))
            } else {
                Box::new(telemetry::get_subscriber(
                    LOGGER_NAME,
                    LOGGER_FILTER_LEVEL,
                    io::sink,
                ))
            };

        telemetry::init_subscriber(subscriber)
    }

    async fn init_db(config: &DatabaseSettings) -> PgPool {
        // Create Database
        let maintenance_settings = DatabaseSettings {
            name: "postgres".into(),
            username: "postgres".into(),
            password: "password".into(),
            ..config.clone()
        };

        PgConnection::connect_with(&maintenance_settings.connect_options())
            .await
            .expect(DB_CONNECTION_FAIL)
            .execute(format!(r#"CREATE DATABASE "{}";"#, config.name).as_str())
            .await
            .expect("Failed to create database");

        // Migrate Database
        let db_pool = PgPool::connect_with(config.connect_options())
            .await
            .expect(DB_CONNECTION_FAIL);

        sqlx::migrate!("./migrations")
            .run(&db_pool)
            .await
            .expect("Failed to migrate the database");

        db_pool
    }
}
