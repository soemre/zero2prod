use crate::helpers::{self, TestApp};

#[tokio::test]
async fn you_must_be_logged_in_to_access_the_admin_dashboard() {
    // Arrange
    let app = TestApp::spawn().await;

    // Act
    let resp = app.get_admin_dashboard().await;

    // Assert
    helpers::assert_redirects_to(&resp, "/login");
}

#[tokio::test]
async fn logout_clears_session_state() {
    // Arrange
    let app = TestApp::spawn().await;
    let body = serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password,
    });

    // Act 1: Login
    let resp = app.post_login(&body).await;
    helpers::assert_redirects_to(&resp, "/admin/dashboard");

    // Act 2: Follow the redirect
    let resp = app.get_admin_dashboard_html().await;
    assert!(resp.contains(&format!("Welcome {}", app.test_user.username)));

    // Act 3: Logout
    let resp = app.post_logout().await;
    helpers::assert_redirects_to(&resp, "/login");

    // Act 4: Follow the redirect
    let resp = app.get_login_html().await;
    assert!(resp.contains(r#"<p><i>You have successfully logged out.</i></p>"#));

    // Act 5: Attempt to load admin panel
    let resp = app.get_admin_dashboard().await;
    helpers::assert_redirects_to(&resp, "/login");
}
