use argon2::{password_hash::SaltString, Algorithm, Argon2, Params, PasswordHasher, Version};
use linkify::{LinkFinder, LinkKind};
use reqwest::{Body, Response, Url};
use serde::Serialize;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::{env, io, net::SocketAddr, sync::LazyLock};
use uuid::Uuid;
use wiremock::{MockServer, Request};
use zero2prod::{
    app::App,
    config::{self, DatabaseSettings},
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

pub struct TestUser {
    pub id: Uuid,
    pub username: String,
    pub password: String,
}

impl TestUser {
    pub fn generate() -> Self {
        Self {
            id: Uuid::new_v4(),
            username: Uuid::new_v4().to_string(),
            password: Uuid::new_v4().to_string(),
        }
    }

    pub async fn store(&self, pool: &PgPool) {
        let password_hash = {
            let salt = SaltString::generate(rand::thread_rng());

            // Eleminate the timing difference that occurs while hashing the password in tests.
            // Need to specify this manually since we are directlly querying the database.
            Argon2::new(
                Algorithm::Argon2id,
                Version::V0x13,
                Params::new(15000, 2, 1, None).unwrap(),
            )
            .hash_password(self.password.as_bytes(), &salt)
            .unwrap()
            .to_string()
        };
        sqlx::query!(
            r#"
            INSERT INTO users (id, username, password_hash)
            VALUES ($1, $2, $3)
            "#,
            self.id,
            self.username,
            password_hash
        )
        .execute(pool)
        .await
        .expect("Failed to store test users.");
    }
}

#[allow(dead_code)]
pub struct TestApp {
    pub base_addr: String,
    pub socket_addr: SocketAddr,
    pub db_pool: PgPool,
    pub email_server: MockServer,
    pub test_user: TestUser,
    pub api_client: reqwest::Client,
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
        let app = App::build(&config)
            .await
            .expect("Failed to build application.");
        let socket_addr = app.addr();
        let base_addr = format!("{}:{}", config.application.base_url, socket_addr.port());
        let api_client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .cookie_store(true)
            .build()
            .unwrap();

        // Run the application as a background task
        tokio::spawn(app.run_until_stopped());

        let test_app = TestApp {
            db_pool: App::get_db_pool(&config.database),
            base_addr,
            email_server,
            socket_addr,
            test_user: TestUser::generate(),
            api_client,
        };
        test_app.test_user.store(&test_app.db_pool).await;
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

    pub async fn post_subscriptions(&self, body: impl Into<Body>) -> Response {
        self.api_client
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
        self.api_client
            .post(format!("{}/newsletters", self.base_addr))
            .basic_auth(&self.test_user.username, Some(&self.test_user.password))
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

    pub async fn post_login<T>(&self, body: &T) -> Response
    where
        T: serde::Serialize + ?Sized,
    {
        self.api_client
            .post(format!("{}/login", self.base_addr))
            .form(body)
            .send()
            .await
            .expect(RQST_FAIL)
    }

    pub async fn get_login_html(&self) -> String {
        self.api_client
            .get(format!("{}/login", self.base_addr))
            .send()
            .await
            .expect(RQST_FAIL)
            .text()
            .await
            .unwrap()
    }

    pub async fn get_admin_dashboard(&self) -> Response {
        self.api_client
            .get(format!("{}/admin/dashboard", self.base_addr))
            .send()
            .await
            .expect(RQST_FAIL)
    }

    pub async fn get_admin_dashboard_html(&self) -> String {
        self.get_admin_dashboard().await.text().await.unwrap()
    }
}

pub fn assert_redirecting(resp: &Response, location: &str) {
    assert_eq!(303, resp.status().as_u16());
    assert_eq!(location, resp.headers().get("Location").unwrap());
}
