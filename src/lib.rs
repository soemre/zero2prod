use actix_web::{dev::Server, get, post, web::Form, App, HttpResponse, HttpServer};

use serde::Deserialize;

use std::{io::Result, net::TcpListener};

#[get("/health_check")]
async fn health_check() -> HttpResponse {
    HttpResponse::Ok().finish()
}

#[derive(Deserialize)]
struct SubscriptionForm {
    name: String,
    email: String,
}

#[post("/subscriptions")]
async fn subscriptions(form: Form<SubscriptionForm>) -> HttpResponse {
    let mut response = if !form.name.is_empty() && !form.email.is_empty() {
        HttpResponse::Ok()
    } else {
        HttpResponse::BadRequest()
    };

    response.finish()
}

pub fn run(listener: TcpListener) -> Result<Server> {
    let server = HttpServer::new(|| {
        return App::new().service(health_check).service(subscriptions);
    })
    .listen(listener)?
    .run();

    Ok(server)
}
