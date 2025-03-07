use crate::helpers::{self, TestApp};

#[tokio::test]
async fn an_error_flash_message_is_set_on_failure() {
    // Arrange
    let app = TestApp::spawn().await;
    let body = serde_json::json!({
        "username": "random-username",
        "password": "random-password",
    });

    // Act: Login
    let resp = app.post_login(&body).await;

    // Assert
    helpers::assert_redirecting(&resp, "/login");

    // Act 2: Follow the redirect
    let html = app.get_login_html().await;

    // Assert 2
    assert!(html.contains(r#"<p><i>Authentication failed</i></p>"#));

    // Act 3: Reload the login page
    let html = app.get_login_html().await;

    // Assert 3
    assert!(!html.contains(r#"<p><i>Authentication failed</i></p>"#));
}

#[tokio::test]
async fn redirect_to_admin_dashboard_after_login_success() {
    // Arrange
    let app = TestApp::spawn().await;
    let body = serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password,
    });

    // Act: Login
    let resp = app.post_login(&body).await;

    // Assert
    helpers::assert_redirecting(&resp, "/admin/dashboard");

    // Act 2: Follow the redirect
    let html = app.get_admin_dashboard_html().await;

    // Assert 2
    assert!(html.contains(&format!("Welcome {}", app.test_user.username)));
}
