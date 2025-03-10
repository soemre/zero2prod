use std::fmt::{Debug, Display};
use tokio::task::JoinError;
use zero2prod::{app::App, config, telemetry, workers::issue_delivery};

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    // binary init - telemetry
    let subscriber = telemetry::get_subscriber("zero2prod", "info", std::io::stdout);
    telemetry::init_subscriber(subscriber);

    // build the app and workers
    let config = config::get().expect("Failed to read configuration");
    let app = {
        let f = App::build(&config).await?.run_until_stopped();
        tokio::spawn(f)
    };
    let worker = {
        let f = issue_delivery::run(config);
        tokio::spawn(f)
    };

    // run concurrently
    tokio::select!(
        o = app => report_exit("API", o),
        o = worker => report_exit("Background worker", o),
    );

    Ok(())
}

fn report_exit(task_name: &str, outcome: Result<Result<(), impl Debug + Display>, JoinError>) {
    match outcome {
        Ok(Ok(_)) => tracing::info!("{} has exited", task_name),
        Ok(Err(e)) => tracing::error!(
                error.cause_chain = ?e,
                error.message = %e,
                "{} failed",
                task_name
        ),
        Err(e) => tracing::error!(
                error.cause_chain = ?e,
                error.message = %e,
                "{} task failed to complete",
                task_name
        ),
    }
}
