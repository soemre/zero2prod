use actix_web::{get, http::header, HttpResponse, Responder};
use actix_web_flash_messages::IncomingFlashMessages;
use std::fmt::Write;

#[get("/newsletters")]
pub async fn newsletters_form(
    flash_messages: IncomingFlashMessages,
) -> actix_web::Result<impl Responder> {
    let mut msg_html = String::new();
    for m in flash_messages.iter() {
        writeln!(msg_html, "<p><i>{}</i></p>", m.content()).unwrap();
    }

    Ok(HttpResponse::Ok()
        .content_type(header::ContentType::html())
        .body(format!(
            r#"
<!DOCTYPE html> 
<html lang="en">
    <head>
        <meta http-equiv="content-type" content="text/html; charset=utf-8">
        <title>Send a Newsletter Issue</title>
    </head>
    <body>
        {msg_html}
        <form action="/admin/newsletters" method="post">
            <label>Title
                <input type="text" placeholder="..." name="title" >
            </label>
            <br>
            <label>Text Content
                <input type="text" placeholder="..." name="text" >
            </label>
            <br>
            <label>HTML Content
                <input type="text" placeholder="..." name="html" >
            </label>
            <br>
            <button type="submit">Publish</button>
        </form>
        <p><a href="/admin/dashboard">&lt;- Back</a></p>
    </body>
</html>
            "#,
        )))
}
