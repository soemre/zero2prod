use actix_web::{get, http::header::ContentType, web, HttpResponse, Responder};

#[derive(serde::Deserialize)]
struct QueryParams {
    error: Option<String>,
}

#[get("/login")]
pub async fn login_form(query: web::Query<QueryParams>) -> impl Responder {
    let error_html = match query.0.error {
        Some(e) => format!("<p><i>{e}</i></p>"),
        None => "".to_string(),
    };
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
