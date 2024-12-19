use actix_web::{dev::Server, get, App, HttpResponse, HttpServer, Responder};

#[get("/health_check")]
async fn health_check() -> impl Responder {
    HttpResponse::Ok()
}

pub fn run(addr: impl std::net::ToSocketAddrs) -> std::io::Result<Server> {
    let server = HttpServer::new(|| {
        return App::new().service(health_check);
    })
    .bind(addr)?
    .run();

    Ok(server)
}
