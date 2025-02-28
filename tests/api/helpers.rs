use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::{env, io, sync::LazyLock};
use uuid::Uuid;
use zero2prod::{
    config::{self, DatabaseSettings},
    startup::App,
    telemetry,
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

        // Randomise configuration to ensure test isolation
        let config = {
            let mut raw = config::get().expect("Failed to read configuration");
            // Use a different database for each test case
            raw.database.name = Uuid::new_v4().to_string();
            // Use a random OS port
            raw.application.port = 0;
            raw
        };

        // Create the database and application
        Self::init_db(&config.database).await;
        let app = App::build(&config).expect("Failed to build application.");
        let addr = format!("http://127.0.0.1:{}", app.addr().port());

        // Run the application as a background task
        tokio::spawn(app.run_until_stopped());

        TestApp {
            db_pool: App::get_db_pool(&config.database),
            addr,
        }
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
