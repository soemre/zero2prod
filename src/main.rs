use zero2prod::{app::App, config, telemetry};

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    // binary init - telemetry
    let subscriber = telemetry::get_subscriber("zero2prod", "info", std::io::stdout);
    telemetry::init_subscriber(subscriber);

    // build the app
    let config = config::get().expect("Failed to read configuration");
    let app = App::build(&config).await?;

    // run the app
    app.run_until_stopped().await
}
