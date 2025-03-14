use crate::{domain::SubscriberEmail, email_client::EmailClient};
use config::{Config, File};
use reqwest::Url;
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use serde_aux::field_attributes::deserialize_number_from_string;
use sqlx::postgres::{PgConnectOptions, PgPool, PgSslMode};
use std::{env, error::Error, time::Duration};

#[derive(Deserialize)]
pub struct Settings {
    pub application: ApplicationSettings,
    pub database: DatabaseSettings,
    pub email_client: EmailClientSettings,
    pub redis_uri: SecretString,
}

#[derive(Deserialize, Clone)]
pub struct ApplicationSettings {
    pub host: String,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub base_url: String,
    pub hmac_secret: SecretString,
}

#[derive(Deserialize, Clone)]
pub struct DatabaseSettings {
    pub username: String,
    pub password: SecretString,
    pub host: String,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub name: String,
    pub require_ssl: bool,
}

impl DatabaseSettings {
    pub fn connect_options(&self) -> PgConnectOptions {
        let ssl_mode = if self.require_ssl {
            PgSslMode::Require
        } else {
            PgSslMode::Prefer
        };

        PgConnectOptions::new()
            .username(&self.username)
            .password(self.password.expose_secret())
            .host(&self.host)
            .port(self.port)
            .database(&self.name)
            .ssl_mode(ssl_mode)
    }

    pub fn get_db_pool(&self) -> PgPool {
        PgPool::connect_lazy_with(self.connect_options())
    }
}

#[derive(Deserialize, Clone)]
pub struct EmailClientSettings {
    pub base_url: String,
    pub sender_email: String,
    pub auth_token: SecretString,
    pub timeout_ms: u64,
}

impl EmailClientSettings {
    pub fn client(&self) -> EmailClient {
        let sender = self.sender().expect("Invalid sender email address.");
        let url = self.url().expect("Invalid base url.");
        let timeout = self.timeout();
        let auth_token = self.auth_token.clone();
        EmailClient::new(url, sender, auth_token, timeout)
    }

    pub fn sender(&self) -> Result<SubscriberEmail, String> {
        SubscriberEmail::parse(self.sender_email.clone())
    }

    pub fn url(&self) -> Result<Url, String> {
        Url::parse(&self.base_url).map_err(|e| e.to_string())
    }

    pub fn timeout(&self) -> Duration {
        Duration::from_millis(self.timeout_ms)
    }
}

pub fn get() -> Result<Settings, Box<dyn Error>> {
    let config_path = env::current_dir()?.join("config");

    let app_env: Environment = env::var("APP_ENV")
        .unwrap_or_else(|_| "local".into())
        .try_into()?;

    let env_file = {
        let mut chars = app_env.as_str().chars();
        let mut env_file = chars.next().unwrap().to_string().to_uppercase();
        env_file.push_str(&chars.collect::<String>());

        format!("{}.toml", env_file)
    };

    let settings = Config::builder()
        .add_source(File::from(config_path.join("Base.toml")))
        .add_source(File::from(config_path.join(env_file)))
        .add_source(
            config::Environment::with_prefix("APP")
                .prefix_separator("__")
                .separator("__"),
        )
        .build()?;

    Ok(settings.try_deserialize::<Settings>()?)
}

pub enum Environment {
    Local,
    Production,
}

impl Environment {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Production => "production",
        }
    }
}

impl TryFrom<String> for Environment {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::try_from(s.as_str())
    }
}

impl TryFrom<&str> for Environment {
    type Error = String;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s.to_lowercase().as_str() {
            _ if s == Self::Production.as_str() => Ok(Self::Production),
            _ if s == Self::Local.as_str() => Ok(Self::Local),
            other => Err(format!(
                "{other} is not a supported environment. \
                Use either `local` or `production`.",
            )),
        }
    }
}
