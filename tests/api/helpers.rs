use linkify::{LinkFinder, LinkKind};
use reqwest::{Body, Client, Response, Url};
use serde::Serialize;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::{env, io, net::SocketAddr, sync::LazyLock};
use uuid::Uuid;
use wiremock::{MockServer, Request};
use zero2prod::{
    config::{self, DatabaseSettings},
    startup::App,
    telemetry,
};

const DB_CONNECTION_FAIL: &str = "Failed to connect to Postgres";
pub const RQST_FAIL: &'static str = "Failed to execute request.";

const LOGGER_NAME: &str = "test";
const LOGGER_FILTER_LEVEL: &str = "info";

static TRACING: LazyLock<()> = LazyLock::new(TestApp::init_logging);

/// Confirmation links embedded in the request to the email API.
pub struct ConfirmationLinks {
    pub html: Url,
    pub text: Url,
}

#[allow(dead_code)]
pub struct TestApp {
    pub base_addr: String,
    pub socket_addr: SocketAddr,
    pub db_pool: PgPool,
    pub email_server: MockServer,
}

impl TestApp {
    /// Runs the app in the background at a random port
    /// and returns the bound address in "http://addr:port" format.
    pub async fn spawn() -> TestApp {
        LazyLock::force(&TRACING);

        let email_server = MockServer::start().await;

        // Randomise configuration to ensure test isolation
        let config = {
            let mut raw = config::get().expect("Failed to read configuration");
            // Use a different database for each test case
            raw.database.name = Uuid::new_v4().to_string();

            // Use a random OS port
            raw.application.port = 0;

            // Replace the email server
            raw.email_client.base_url = email_server.uri();

            raw
        };

        // Create the database and application
        Self::init_db(&config.database).await;
        let app = App::build(&config).expect("Failed to build application.");
        let socket_addr = app.addr();
        let base_addr = format!("{}:{}", config.application.base_url, socket_addr.port());

        // Run the application as a background task
        tokio::spawn(app.run_until_stopped());

        let test_app = TestApp {
            db_pool: App::get_db_pool(&config.database),
            base_addr,
            email_server,
            socket_addr,
        };
        test_app.add_test_user().await;
        test_app
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

    async fn add_test_user(&self) {
        sqlx::query!(
            r#"
            INSERT INTO users (id, username, password)
            VALUES ($1, $2, $3)
            "#,
            Uuid::new_v4(),
            Uuid::new_v4().to_string(),
            Uuid::new_v4().to_string(),
        )
        .execute(&self.db_pool)
        .await
        .expect("Failed to create test users.");
    }

    pub async fn test_user(&self) -> (String, String) {
        let r = sqlx::query!(
            r#"
        SELECT username, password FROM users LIMIT 1
        "#
        )
        .fetch_one(&self.db_pool)
        .await
        .expect("Failed to fetch the test user.");

        (r.username, r.password)
    }

    pub async fn post_subscriptions(&self, body: impl Into<Body>) -> Response {
        Client::new()
            .post(format!("{}/subscriptions", self.base_addr))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect(RQST_FAIL)
    }

    pub async fn post_newsletters<T>(&self, json: &T) -> Response
    where
        T: Serialize + ?Sized,
    {
        let (username, password) = self.test_user().await;
        Client::new()
            .post(format!("{}/newsletters", self.base_addr))
            .basic_auth(username, Some(password))
            .json(json)
            .send()
            .await
            .expect(RQST_FAIL)
    }

    /// Extract the confirmation links embedded in the request to the email API.
    pub fn get_confirmation_links(&self, email_request: &Request) -> ConfirmationLinks {
        let body: serde_json::Value = email_request.body_json().unwrap();

        let get_link = |s: &str| {
            let links: Vec<_> = LinkFinder::new().kinds(&[LinkKind::Url]).links(s).collect();
            assert_eq!(1, links.len());
            let raw = links[0].as_str().to_owned();

            let mut url = Url::parse(&raw).unwrap();

            // Make sure not to call random APIs on the web
            assert_eq!(url.host_str().unwrap(), "127.0.0.1");

            // Test Harness specific tweaking
            url.set_port(Some(self.socket_addr.port())).unwrap();
            url
        };

        let html = get_link(body["HtmlBody"].as_str().unwrap());
        let text = get_link(body["TextBody"].as_str().unwrap());

        ConfirmationLinks { html, text }
    }
}
