use sqlx::PgPool;
use std::{io::Result, net::TcpListener};
use zero2prod::{config, email_client::EmailClient, startup::run, telemetry};

#[actix_web::main]
async fn main() -> Result<()> {
    // telemetry
    let subscriber = telemetry::get_subscriber("zero2prod", "info", std::io::stdout);
    telemetry::init_subscriber(subscriber);

    // config
    let config = config::get().expect("Failed to read configuration");

    // create the app dependencies
    let listener = TcpListener::bind((config.application.host, config.application.port))?;
    let db_conn = PgPool::connect_lazy_with(config.database.connect_options());
    let email_client = {
        let ec = config.email_client;
        let sender = ec.sender().expect("Invalid sender email address.");
        let url = ec.url().expect("Invalid base url.");
        let auth_token = ec.auth_token;
        EmailClient::new(url, sender, auth_token)
    };

    run(listener, db_conn, email_client)?.await
}
