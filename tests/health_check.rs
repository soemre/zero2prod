const HOST_ADDR: &str = "http://127.0.0.1:8000";

#[tokio::test]
async fn health_check_works() {
    spawn_app();

    let client = reqwest::Client::new();

    let response = client
        .get(format!("{HOST_ADDR}/health_check"))
        .send()
        .await
        .expect("Failed to execute request.");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length())
}

use tokio;

fn spawn_app() {
    todo!();
    tokio::spawn(zero2prod::run().expect("Failed to bind address."));
}
