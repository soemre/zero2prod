use actix_web::{dev::Server, App, HttpServer};
use std::{io::Result, net::TcpListener};

use crate::routes;

pub fn run(listener: TcpListener) -> Result<Server> {
    let server = HttpServer::new(|| {
        return App::new()
            .service(routes::health_check)
            .service(routes::subscribe);
    })
    .listen(listener)?
    .run();

    Ok(server)
}
