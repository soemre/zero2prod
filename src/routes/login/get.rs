use actix_web::{get, http::header::ContentType, HttpResponse, Responder};
use actix_web_flash_messages::{IncomingFlashMessages, Level};
use std::fmt::Write;

#[get("/login")]
pub async fn login_form(flash_messages: IncomingFlashMessages) -> impl Responder {
    let mut error_html = String::new();
    for m in flash_messages.iter().filter(|m| m.level() == Level::Error) {
        let m = htmlescape::encode_minimal(m.content());
        writeln!(error_html, "<p><i>{}</i></p>", m).unwrap();
    }
    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(
            r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Login</title>
</head>
<body>
    {error_html}
    <form action="/login" method="post">
        <label>Username
                <input type="text" name="username" placeholder="Enter Username">
        </label>

        <label>Password
                <input type="password" name="password" placeholder="Enter Password">
        </label>

        <button type="submit">Login</button>
    </form>
</body>
</html>
        "#
        ))
}
