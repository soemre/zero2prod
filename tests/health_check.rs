use reqwest::Client;
use std::net::TcpListener;
use tokio;

const HOST_ADDR: (&str, u16) = ("127.0.0.1", 0);

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

/// Runs the app in the background at a random port
/// and returns the bound address in "http://addr:port" format.
fn spawn_app() -> String {
    let listener = TcpListener::bind(HOST_ADDR).expect("Failed to bind address.");
    let addr = listener.local_addr().unwrap();
    let server = zero2prod::run(listener).expect("Failed to run the server.");
    tokio::spawn(server);

    format!("http://{}:{}", addr.ip(), addr.port())
}

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    // Arrange
    let addr = spawn_app();
    let client = Client::new();
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    // Act
    let response = client
        .post(format!("{addr}/subscriptions"))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert_eq!(200, response.status().as_u16())
}

#[tokio::test]
async fn subscribe_returns_a_400_when_data_is_missing() {
    // Arrange
    let addr = spawn_app();
    let client = Client::new();
    let test_cases = vec![
        ("name=le%20guin", "missing the email"),
        ("email=ursula_le_guin%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (invalid_body, error_message) in test_cases {
        // Act
        let response = client
            .post(format!("{addr}/subscriptions"))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(invalid_body)
            .send()
            .await
            .expect("Failed to execute request.");

        // Assert
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the payload was {}.",
            error_message,
        )
    }
}
