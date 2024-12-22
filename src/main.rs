use zero2prod::{config, startup::run};

use std::{io::Result, net::TcpListener};

#[actix_web::main]
async fn main() -> Result<()> {
    let config = config::get().expect("Failed to read configuration");

    let addr: (&str, u16) = ("127.0.0.1", config.application.port);
    let listener = TcpListener::bind(addr)?;
    run(listener)?.await
}
