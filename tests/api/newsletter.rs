use crate::helpers::{self, TestApp};
use std::time::Duration;
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
        "idempotency_key": uuid::Uuid::new_v4(),
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
        "idempotency_key": uuid::Uuid::new_v4(),
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
        "idempotency_key": uuid::Uuid::new_v4(),
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

#[tokio::test]
async fn newsletter_creation_is_idempotent() {
    // Arrange
    let app = TestApp::spawn().await;
    app.create_confirmed_subscriber().await;
    app.login_as_test_user().await;

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
        "idempotency_key": uuid::Uuid::new_v4(),
    });

    // Act 1: Send the form
    let resp = app.post_newsletters(&newsletter_request_body).await;
    helpers::assert_redirects_to(&resp, "/admin/newsletters");

    // Act 2: Follow the redirection
    let html = app.get_newsletters_html().await;
    assert!(html.contains("<p><i>All done! The newsletter has been published.</i></p>"));

    // Act 3: Send the form again
    let resp = app.post_newsletters(&newsletter_request_body).await;
    helpers::assert_redirects_to(&resp, "/admin/newsletters");

    // Act 4: Follow the redirection
    let html = app.get_newsletters_html().await;
    assert!(html.contains("<p><i>All done! The newsletter has been published.</i></p>"));
}

#[tokio::test]
async fn concurrent_form_submission_is_handled_gracefully() {
    // Arrange
    let app = TestApp::spawn().await;
    app.create_confirmed_subscriber().await;
    app.login_as_test_user().await;

    Mock::given(matchers::path("/email"))
        .and(matchers::method("POST"))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(2)))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "text": "Newsletter body as plain text",
        "html": "<p>Newsletter body as HTML</p>",
        "idempotency_key": uuid::Uuid::new_v4(),
    });

    // Act
    let resp1 = app.post_newsletters(&newsletter_request_body);
    let resp2 = app.post_newsletters(&newsletter_request_body);
    let (resp1, resp2) = tokio::join!(resp1, resp2);

    // Assert
    assert_eq!(resp1.status(), resp2.status());
    assert_eq!(resp1.text().await.unwrap(), resp2.text().await.unwrap());
}
