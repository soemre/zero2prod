use sqlx::PgPool;
use std::{io::Result, net::TcpListener};
use zero2prod::{config, startup::run};

#[actix_web::main]
async fn main() -> Result<()> {
    let config = config::get().expect("Failed to read configuration");
    let listener = TcpListener::bind(("127.0.0.1", config.application.port))?;
    let db_conn = PgPool::connect(&config.database.url())
        .await
        .expect("Failed to connect to Postgres");

    run(listener, db_conn)?.await
}
