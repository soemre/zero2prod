use secrecy::ExposeSecret;
use sqlx::PgPool;
use std::{io::Result, net::TcpListener};
use zero2prod::{config, startup::run, telemetry};

#[actix_web::main]
async fn main() -> Result<()> {
    let subscriber = telemetry::get_subscriber("zero2prod", "info", std::io::stdout);
    telemetry::init_subscriber(subscriber);

    let config = config::get().expect("Failed to read configuration");
    let listener = TcpListener::bind((config.application.host, config.application.port))?;
    let db_conn = PgPool::connect_lazy(&config.database.url().expose_secret())
        .expect("Failed to connect to Postgres");

    run(listener, db_conn)?.await
}
