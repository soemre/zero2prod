use crate::{session_state::Session, utils};
use actix_web::{post, Responder};
use actix_web_flash_messages::FlashMessage;

#[post("/logout")]
async fn logout(session: Session) -> actix_web::Result<impl Responder> {
    session.logout();
    FlashMessage::info("You have successfully logged out.").send();
    Ok(utils::see_other("/login"))
}
