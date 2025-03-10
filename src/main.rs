use std::fmt::{Debug, Display};
use tokio::task::JoinError;
use zero2prod::{
    app::App,
    config, telemetry,
    workers::{expiration, issue_delivery},
};

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
    let issue_delivery_worker = {
        let f = issue_delivery::Worker::builder(&config).finish();
        tokio::spawn(f)
    };
    let expiration_worker = {
        let f = expiration::Worker::builder(&config).finish();
        tokio::spawn(f)
    };

    // run concurrently
    tokio::select!(
        o = app => report_exit("API", o),
        o = expiration_worker => report_exit("Expiration Background Worker", o),
        o = issue_delivery_worker => report_exit("Issue Delivery Background Worker", o),
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
