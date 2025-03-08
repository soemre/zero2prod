use crate::{
    auth::{self, AuthError, Credentials},
    session_state::Session,
    utils,
};
use actix_web::{error::InternalError, http::header, post, web, HttpResponse, Responder};
use actix_web_flash_messages::FlashMessage;
use secrecy::SecretString;
use sqlx::PgPool;
use std::fmt::Debug;

#[derive(serde::Deserialize)]
pub struct FormData {
    username: String,
    password: SecretString,
}

#[post("/login")]
#[tracing::instrument(skip(form, pool, session), fields(username = tracing::field::Empty, user_id = tracing::field::Empty))]
pub async fn login(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
    session: Session,
) -> Result<impl Responder, InternalError<LoginError>> {
    let credentials = Credentials {
        username: form.0.username,
        password: form.0.password,
    };
    tracing::Span::current().record("username", tracing::field::display(&credentials.username));

    match auth::validate_credentials(credentials, pool.as_ref()).await {
        Ok(id) => {
            session.renew();
            session
                .user_id()
                .insert(id)
                .map_err(|e| login_redirect(LoginError::UnexpectedError(e.into())))?;
            tracing::Span::current().record("user_id", tracing::field::display(&id));
            Ok(utils::see_other("/admin/dashboard"))
        }
        Err(e) => {
            let e = match e {
                AuthError::InvalidCredentials(_) => LoginError::AuthError(e.into()),
                AuthError::UnexpectedError(_) => LoginError::UnexpectedError(e.into()),
            };
            Err(login_redirect(e))
        }
    }
}

/// Redirect to the login page with an error message.
fn login_redirect(e: LoginError) -> InternalError<LoginError> {
    FlashMessage::error(e.to_string()).send();
    let resp = HttpResponse::SeeOther()
        .insert_header((header::LOCATION, "/login"))
        .finish();

    InternalError::from_response(e, resp)
}

#[derive(thiserror::Error)]
enum LoginError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error("Something went wrong")]
    UnexpectedError(#[from] anyhow::Error),
}

impl Debug for LoginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        utils::error_chain_fmt(self, f)
    }
}
