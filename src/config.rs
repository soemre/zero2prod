use config::{Config, ConfigError, File, FileFormat};
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Settings {
    pub application: ApplicationSettings,
    pub database: DatabaseSettings,
}

#[derive(Deserialize, Clone)]
pub struct ApplicationSettings {
    pub port: u16,
}

#[derive(Deserialize, Clone)]
pub struct DatabaseSettings {
    pub username: String,
    pub password: SecretString,
    pub host: String,
    pub port: u16,
    pub name: String,
}

impl DatabaseSettings {
    pub fn url(&self) -> SecretString {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username,
            self.password.expose_secret(),
            self.host,
            self.port,
            self.name
        )
        .into()
    }
}

pub fn get() -> Result<Settings, ConfigError> {
    let settings = Config::builder()
        .add_source(File::new("App.toml", FileFormat::Toml))
        .build()?;

    settings.try_deserialize()
}
