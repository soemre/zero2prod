use crate::domain::SubscriberEmail;
use config::{Config, File};
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use serde_aux::field_attributes::deserialize_number_from_string;
use sqlx::postgres::{PgConnectOptions, PgSslMode};
use std::{env, error::Error};

#[derive(Deserialize)]
pub struct Settings {
    pub application: ApplicationSettings,
    pub database: DatabaseSettings,
    pub email_client: EmailClientSettings,
}

#[derive(Deserialize, Clone)]
pub struct ApplicationSettings {
    pub host: String,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
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
            .password(&self.password.expose_secret())
            .host(&self.host)
            .port(self.port)
            .database(&self.name)
            .ssl_mode(ssl_mode)
    }
}

#[derive(Deserialize, Clone)]
pub struct EmailClientSettings {
    pub base_url: String,
    pub sender_email: String,
}

impl EmailClientSettings {
    pub fn sender(&self) -> Result<SubscriberEmail, String> {
        SubscriberEmail::parse(self.sender_email.clone())
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
