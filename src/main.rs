use zero2prod::run;

use std::{io::Result, net::TcpListener};

const ADDR: (&str, u16) = ("127.0.0.1", 8000);

#[actix_web::main]
async fn main() -> Result<()> {
    let listener = TcpListener::bind(ADDR)?;
    run(listener)?.await
}
