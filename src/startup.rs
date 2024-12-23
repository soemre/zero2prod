use actix_web::{dev::Server, web::Data, App, HttpServer};
use sqlx::PgPool;
use std::{io::Result, net::TcpListener};

use crate::routes;

pub fn run(listener: TcpListener, db_pool: PgPool) -> Result<Server> {
    let db_pool = Data::new(db_pool);
    let server = HttpServer::new(move || {
        return App::new()
            .service(routes::health_check)
            .service(routes::subscribe)
            .app_data(Data::clone(&db_pool));
    })
    .listen(listener)?
    .run();

    Ok(server)
}
