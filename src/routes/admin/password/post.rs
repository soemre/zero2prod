use crate::{
    auth::{self, AuthError, Credentials, UserId},
    domain::ValidPassword,
    routes::admin::dashboard::get_username,
    utils,
};
use actix_web::{post, web, Responder};
use actix_web_flash_messages::FlashMessage;
use secrecy::{ExposeSecret, SecretString};
use sqlx::PgPool;

#[derive(serde::Deserialize)]
struct FormData {
    current_password: SecretString,
    new_password: SecretString,
    new_password_check: SecretString,
}

#[post("/password")]
async fn change_password(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
) -> actix_web::Result<impl Responder> {
    let user_id = user_id.into_inner();

    if form.new_password.expose_secret() != form.new_password_check.expose_secret() {
        FlashMessage::error(
            "You entered two different new passwords - the field values must match.",
        )
        .send();
        return Ok(utils::see_other("/admin/password"));
    }

    let new_password = match ValidPassword::parse(form.0.new_password) {
        Ok(p) => p,
        Err(e) => {
            FlashMessage::error(e.to_string()).send();
            return Ok(utils::see_other("/admin/password"));
        }
    };

    let credentials = {
        let username = get_username(*user_id, pool.as_ref())
            .await
            .map_err(utils::e500)?;
        Credentials {
            username,
            password: form.0.current_password,
        }
    };

    if let Err(e) = auth::validate_credentials(credentials, pool.as_ref()).await {
        return match e {
            AuthError::UnexpectedError(_) => Err(utils::e500(e)),
            AuthError::InvalidCredentials(_) => {
                FlashMessage::error("The current password is incorrect.").send();
                Ok(utils::see_other("/admin/password"))
            }
        };
    }

    auth::change_password(*user_id, new_password, pool.as_ref())
        .await
        .map_err(utils::e500)?;

    FlashMessage::info("Your password has been changed.").send();
    Ok(utils::see_other("/admin/password"))
}
