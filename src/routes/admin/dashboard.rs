use crate::{session_state::Session, utils};
use actix_web::{get, http::header, web, HttpResponse, Responder};
use anyhow::Context;
use sqlx::{PgExecutor, PgPool};
use uuid::Uuid;

#[get("/admin/dashboard")]
pub async fn admin_dashboard(
    session: Session,
    pool: web::Data<PgPool>,
) -> actix_web::Result<impl Responder> {
    let username = if let Some(id) = session.user_id().get().map_err(utils::e500)? {
        get_username(id, pool.as_ref()).await.map_err(utils::e500)?
    } else {
        return Ok(HttpResponse::SeeOther()
            .insert_header((header::LOCATION, "/login"))
            .finish());
    };
    Ok(HttpResponse::Ok()
        .content_type(header::ContentType::html())
        .body(format!(
            r#"
<!DOCTYPE html>
<html lang="en">
    <head>
        <meta http-equiv="content-type" content="text/html; charset=utf-8">
        <title>Admin dashboard</title>
    </head>
    <body>
        <p>Welcome {username}!</p>
        <p>Available actions:</p>
        <ol>
            <li><a href="/admin/password">Change password</a></li>
            <li>
                <form name="logoutForm" action="/admin/logout" method="post">
                    <input type="submit" value="Logout">
                </form>
            </li>
        </ol>
    </body>
</html>
"#
        )))
}

#[tracing::instrument(name = "Get username", skip(executor))]
pub async fn get_username(user_id: Uuid, executor: impl PgExecutor<'_>) -> anyhow::Result<String> {
    let r = sqlx::query!(
        r#"
    SELECT username
    FROM users
    WHERE id = $1
    "#,
        user_id
    )
    .fetch_one(executor)
    .await
    .context("Failed to perform a query to retrieve a username.")?;

    Ok(r.username)
}
