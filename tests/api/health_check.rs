use crate::helpers::*;
use reqwest::Client;

#[tokio::test]
async fn health_check_works() {
    // Arrange
    let app = TestApp::spawn().await;
    let client = Client::new();

    // Act
    let response = client
        .get(format!("{}/health_check", app.base_addr))
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length())
}
