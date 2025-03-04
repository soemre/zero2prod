use crate::helpers::{ConfirmationLinks, TestApp};
use wiremock::{matchers, Mock, ResponseTemplate};

#[tokio::test]
async fn newsletters_returns_400_for_invalid_datanewsletters_returns_400_for_invalid_data() {
    // Arrange
    let app = TestApp::spawn().await;
    let test_cases = vec![
        (
            serde_json::json!({ "content": { "text": "Newsletter body as plain text", "html": "<p>Newsletter body as HTML</p>", } }),
            "missing title",
        ),
        (
            serde_json::json!({"title": "Newsletter!"}),
            "missing content",
        ),
    ];

    for (body, err_msg) in test_cases {
        // Act
        let resp = app.post_newsletters(&body).await;

        // Assert
        assert_eq!(
            400,
            resp.status().as_u16(),
            "The API did not fail with 400 Bad Request when the payload was {}.",
            err_msg
        );
    }
}

#[tokio::test]
async fn newsletters_are_not_delivered_to_unconfirmed_subscribers() {
    // Arrange
    let app = TestApp::spawn().await;
    create_unconfirmed_subscriber(&app).await;

    Mock::given(matchers::any())
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&app.email_server)
        .await;

    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "content": {
            "text": "Newsletter body as plain text",
            "html": "<p>Newsletter body as HTML</p>",
        },
    });

    // Act
    let resp = app.post_newsletters(&newsletter_request_body).await;

    // Assert
    assert_eq!(200, resp.status().as_u16());
}

#[tokio::test]
async fn newsletters_are_delivered_to_confirmed_subscribers() {
    // Arrange
    let app = TestApp::spawn().await;
    create_confirmed_subscriber(&app).await;

    Mock::given(matchers::path("/email"))
        .and(matchers::method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "content": {
            "text": "Newsletter body as plain text",
            "html": "<p>Newsletter body as HTML</p>",
        },
    });

    // Act
    let resp = app.post_newsletters(&newsletter_request_body).await;

    // Assert
    assert_eq!(200, resp.status().as_u16());
}

async fn create_unconfirmed_subscriber(app: &TestApp) -> ConfirmationLinks {
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    let _mock_guard = Mock::given(matchers::path("/email"))
        .and(matchers::method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .named("Create unconfirmed subscriber")
        .expect(1)
        .mount_as_scoped(&app.email_server)
        .await;

    app.post_subscriptions(body)
        .await
        .error_for_status()
        .unwrap();

    let email_request = &app.email_server.received_requests().await.unwrap()[0];

    app.get_confirmation_links(email_request)
}

async fn create_confirmed_subscriber(app: &TestApp) {
    let confirmation_link = create_unconfirmed_subscriber(app).await.text;

    reqwest::get(confirmation_link)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
}
