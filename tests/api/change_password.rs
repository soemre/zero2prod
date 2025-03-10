use crate::helpers::{self, TestApp};
use uuid::Uuid;

#[tokio::test]
async fn you_must_be_logged_in_to_see_the_change_password_form() {
    // Arrange
    let app = TestApp::spawn().await;

    // Act
    let resp = app.get_change_password().await;

    // Assert
    helpers::assert_redirects_to(&resp, "/login")
}

#[tokio::test]
async fn you_must_be_logged_in_to_change_your_password() {
    // Arrange
    let app = TestApp::spawn().await;
    let new_password = Uuid::new_v4();

    // Act
    let resp = app
        .post_change_password(&serde_json::json!({
            "current_password": Uuid::new_v4(),
            "new_password": &new_password,
            "new_password_check": &new_password,
        }))
        .await;

    // Assert
    helpers::assert_redirects_to(&resp, "/login")
}

#[tokio::test]
async fn new_password_fields_must_match() {
    // Arrange
    let app = TestApp::spawn().await;
    let new_password = Uuid::new_v4();
    let new_password_check = Uuid::new_v4();

    // Act 1: Login
    app.post_login(&serde_json::json!({
            "username": &app.test_user.username,
            "password": &app.test_user.password,
    }))
    .await;

    // Act 2: Try to change the password
    let resp = app
        .post_change_password(&serde_json::json!({
            "current_password": &app.test_user.password,
            "new_password": &new_password,
            "new_password_check": &new_password_check,
        }))
        .await;

    // Assert
    helpers::assert_redirects_to(&resp, "/admin/password");

    // Act 3: Follow the redirect
    let html = app.get_change_password_html().await;
    assert!(html.contains(
        "<p><i>You entered two different new passwords - the field values must match.</i></p>"
    ))
}

#[tokio::test]
async fn current_password_must_be_valid() {
    // Arrange
    let app = TestApp::spawn().await;
    let new_password = Uuid::new_v4();

    // Act 1: Login
    app.post_login(&serde_json::json!({
            "username": &app.test_user.username,
            "password": &app.test_user.password,
    }))
    .await;

    // Act 2: Try to change the password
    let resp = app
        .post_change_password(&serde_json::json!({
            "current_password": Uuid::new_v4(),
            "new_password": &new_password,
            "new_password_check": &new_password,
        }))
        .await;

    // Assert
    helpers::assert_redirects_to(&resp, "/admin/password");

    // Act 3: Follow the redirect
    let html = app.get_change_password_html().await;
    assert!(html.contains("<p><i>The current password is incorrect.</i></p>"))
}

use fake::{Fake, StringFaker};

#[tokio::test]
async fn rejects_invalid_new_passwords() {
    // Arrange
    let app = TestApp::spawn().await;

    const ASCII: &str =
        "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ!\"#$%&\'()*+,-./:;<=>?@";

    let new_passwords: Vec<(String, &str)> = vec![
        (
            StringFaker::with(ASCII.into(), 0..=12).fake(),
            "under 12 characters",
        ),
        (
            StringFaker::with(ASCII.into(), 130).fake(),
            "130 characters",
        ),
    ];

    // Act 1: Login
    app.post_login(&serde_json::json!({
            "username": &app.test_user.username,
            "password": &app.test_user.password,
    }))
    .await;

    for p in new_passwords {
        // Act 2: Try to change the password
        app.post_change_password(&serde_json::json!({
            "current_password": app.test_user.password,
            "new_password": &p.0,
            "new_password_check": &p.0,
        }))
        .await;

        // Act 3: Follow the redirect
        let html = app.get_change_password_html().await;

        // Assert
        assert!(html.contains("<p><i>Passwords must be longer than 12 characters but shorter than 129 characters.</i></p>"), 
        "The API didn't included an error message when the new password was {}", p.1)
    }
}

#[tokio::test]
async fn changing_password_works() {
    // Arrange
    let app = TestApp::spawn().await;
    let new_password = Uuid::new_v4();

    // Act 1: Login
    app.login_as_test_user().await;

    // Act 2: Change the password
    let resp = app
        .post_change_password(&serde_json::json!({
            "current_password": &app.test_user.password,
            "new_password": &new_password,
            "new_password_check": &new_password,
        }))
        .await;
    helpers::assert_redirects_to(&resp, "/admin/password");

    // Act 3: Follow the redirect
    let html = app.get_change_password_html().await;
    assert!(html.contains("<p><i>Your password has been changed.</i></p>"));

    // Act 4: Logout
    let resp = app.post_logout().await;
    helpers::assert_redirects_to(&resp, "/login");

    // Act 5: Follow the redirect
    let resp = app.get_login_html().await;
    assert!(resp.contains(r#"<p><i>You have successfully logged out.</i></p>"#));

    // Act 6: Login using the new password
    let resp = app
        .post_login(&serde_json::json!({
                "username": &app.test_user.username,
                "password": &new_password,
        }))
        .await;
    helpers::assert_redirects_to(&resp, "/admin/dashboard");
}
