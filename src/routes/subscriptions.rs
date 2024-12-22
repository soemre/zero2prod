use actix_web::{post, web::Form, HttpResponse};

use serde::Deserialize;

#[derive(Deserialize)]
pub struct SubscriptionForm {
    name: String,
    email: String,
}

#[post("/subscriptions")]
pub async fn subscribe(form: Form<SubscriptionForm>) -> HttpResponse {
    let mut response = if !form.name.is_empty() && !form.email.is_empty() {
        HttpResponse::Ok()
    } else {
        HttpResponse::BadRequest()
    };

    response.finish()
}
