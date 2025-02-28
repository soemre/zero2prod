use std::io::Result;
use zero2prod::{config, startup::App, telemetry};

#[actix_web::main]
async fn main() -> Result<()> {
    // binary init - telemetry
    let subscriber = telemetry::get_subscriber("zero2prod", "info", std::io::stdout);
    telemetry::init_subscriber(subscriber);

    // build the app
    let config = config::get().expect("Failed to read configuration");
    let app = App::build(&config)?;

    // run the app
    app.run_until_stopped().await
}
