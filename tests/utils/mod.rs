use std::net::TcpListener;

/// Runs the app in the background at a random port
/// and returns the bound address in "http://addr:port" format.
pub fn spawn_app() -> String {
    const HOST_ADDR: (&str, u16) = ("127.0.0.1", 0);
    let listener = TcpListener::bind(HOST_ADDR).expect("Failed to bind address.");
    let addr = listener.local_addr().unwrap();
    let server = zero2prod::run(listener).expect("Failed to run the server.");
    tokio::spawn(server);

    format!("http://{}:{}", addr.ip(), addr.port())
}
