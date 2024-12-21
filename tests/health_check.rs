use reqwest::Client;
use tokio;

mod utils;
use utils::*;

#[tokio::test]
async fn health_check_works() {
    // Arrange
    let addr = spawn_app();
    let client = Client::new();

    // Act
    let response = client
        .get(format!("{addr}/health_check"))
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length())
}
