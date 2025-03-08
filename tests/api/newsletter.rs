use crate::helpers::{self, TestApp};
use wiremock::{matchers, Mock, ResponseTemplate};

#[tokio::test]
async fn newsletters_are_not_delivered_to_unconfirmed_subscribers() {
    // Arrange
    let app = TestApp::spawn().await;
    app.create_unconfirmed_subscriber().await;

    Mock::given(matchers::any())
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&app.email_server)
        .await;

    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "text": "Newsletter body as plain text",
        "html": "<p>Newsletter body as HTML</p>",
    });

    app.login_as_test_user().await;

    // Act: Publish a newsletter
    let resp = app.post_newsletters(&newsletter_request_body).await;
    helpers::assert_redirects_to(&resp, "/admin/newsletters");

    // Act: Follow the redirection
    let html = app.get_newsletters_html().await;
    assert!(html.contains("<p><i>All done! The newsletter has been published.</i></p>"));
}

#[tokio::test]
async fn newsletters_are_delivered_to_confirmed_subscribers() {
    // Arrange
    let app = TestApp::spawn().await;
    app.create_confirmed_subscriber().await;

    Mock::given(matchers::path("/email"))
        .and(matchers::method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "text": "Newsletter body as plain text",
        "html": "<p>Newsletter body as HTML</p>",
    });

    app.login_as_test_user().await;

    // Act: Publish a newsletter
    let resp = app.post_newsletters(&newsletter_request_body).await;
    helpers::assert_redirects_to(&resp, "/admin/newsletters");

    // Act: Follow the redirection
    let html = app.get_newsletters_html().await;
    assert!(html.contains("<p><i>All done! The newsletter has been published.</i></p>"));
}

#[tokio::test]
async fn you_must_be_logged_in_to_issue_newsletters() {
    // Arrange
    let app = TestApp::spawn().await;

    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "text": "Newsletter body as plain text",
        "html": "<p>Newsletter body as HTML</p>",
    });

    // Act
    let resp = app.post_newsletters(&newsletter_request_body).await;

    // Assert
    helpers::assert_redirects_to(&resp, "/login");
}

#[tokio::test]
async fn you_must_be_logged_in_to_see_the_newsletter_form() {
    // Arrange
    let app = TestApp::spawn().await;

    // Act
    let resp = app.get_newsletters().await;

    // Assert
    helpers::assert_redirects_to(&resp, "/login");
}
