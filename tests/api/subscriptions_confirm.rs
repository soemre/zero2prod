use crate::helpers::TestApp;
use wiremock::{matchers, Mock, ResponseTemplate};

#[tokio::test]
async fn confirmations_without_token_are_rejected_with_a_400() {
    // Arrange
    let app = TestApp::spawn().await;

    // Act
    let response = reqwest::get(format!("{}/subscriptions/confirm", app.addr))
        .await
        .unwrap();

    // Assert
    assert_eq!(400, response.status().as_u16())
}

#[tokio::test]
async fn the_link_returned_by_subscribe_returns_a_200_if_called() {
    // Arrange
    let app = TestApp::spawn().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(matchers::path("/email"))
        .and(matchers::method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body).await;

    let link = {
        let email_request = &app.email_server.received_requests().await.unwrap()[0];
        app.get_confirmation_links(email_request).html
    };

    // Act
    let response = reqwest::get(link).await.unwrap();

    // Assert
    assert_eq!(response.status().as_u16(), 200)
}

#[tokio::test]
async fn clicking_on_the_confirmation_link_confirms_a_subscriber() {
    // Arrange
    let app = TestApp::spawn().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(matchers::path("/email"))
        .and(matchers::method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body).await;

    let link = {
        let email_request = &app.email_server.received_requests().await.unwrap()[0];
        app.get_confirmation_links(email_request).html
    };

    // Act
    reqwest::get(link)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    // Assert
    let saved = sqlx::query!("SELECT email, name, status FROM subscriptions")
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscription.");

    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
    assert_eq!(saved.status, "confirmed");
}
