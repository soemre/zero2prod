use zero2prod::run;

use std::io::Result;

const ADDR: (&str, u16) = ("127.0.0.1", 8000);

#[actix_web::main]
async fn main() -> Result<()> {
    run(ADDR)?.await
}
