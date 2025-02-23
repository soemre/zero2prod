use crate::routes;
use actix_web::{dev::Server, web::Data, App, HttpServer};
use sqlx::PgPool;
use std::{io::Result, net::TcpListener};
use tracing_actix_web::TracingLogger;

pub fn run(listener: TcpListener, db_pool: PgPool) -> Result<Server> {
    let db_pool = Data::new(db_pool);
    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .service(routes::health_check)
            .service(routes::subscribe)
            .app_data(Data::clone(&db_pool))
    })
    .listen(listener)?
    .run();

    Ok(server)
}
