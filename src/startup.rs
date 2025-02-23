use crate::{email_client::EmailClient, routes};
use actix_web::{dev::Server, web::Data, App, HttpServer};
use sqlx::PgPool;
use std::{io::Result, net::TcpListener};
use tracing_actix_web::TracingLogger;

pub fn run(listener: TcpListener, db_pool: PgPool, email_client: EmailClient) -> Result<Server> {
    let db_pool = Data::new(db_pool);
    let email_client = Data::new(email_client);
    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .service(routes::health_check)
            .service(routes::subscribe)
            .app_data(Data::clone(&db_pool))
            .app_data(Data::clone(&email_client))
    })
    .listen(listener)?
    .run();

    Ok(server)
}
