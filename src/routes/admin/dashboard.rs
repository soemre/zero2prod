use crate::session_state::SessionState;
use actix_web::{get, http::header, web, HttpResponse, Responder};
use anyhow::Context;
use sqlx::{PgExecutor, PgPool};
use std::fmt::{Debug, Display};
use uuid::Uuid;

#[get("/admin/dashboard")]
pub async fn admin_dashboard(
    session: SessionState,
    pool: web::Data<PgPool>,
) -> actix_web::Result<impl Responder> {
    let username = if let Some(id) = session.user_id().get().map_err(e500)? {
        get_username(id, pool.as_ref()).await.map_err(e500)?
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
    </body>
</html>
"#
        )))
}

fn e500(e: impl Debug + Display + 'static) -> actix_web::Error {
    actix_web::error::ErrorInternalServerError(e)
}

#[tracing::instrument(name = "Get username", skip(executor))]
async fn get_username(user_id: Uuid, executor: impl PgExecutor<'_>) -> anyhow::Result<String> {
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
