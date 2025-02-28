use crate::{
    config::{DatabaseSettings, Settings},
    email_client::EmailClient,
    routes,
};
use actix_web::{dev::Server, web::Data, HttpServer};
use core::net::SocketAddr;
use sqlx::PgPool;
use std::{io::Result, net::TcpListener};
use tracing_actix_web::TracingLogger;

pub struct App {
    server: Server,
    socket_addr: SocketAddr,
}

impl App {
    pub fn build(config: &Settings) -> Result<Self> {
        // create the app dependencies
        let listener =
            TcpListener::bind((config.application.host.clone(), config.application.port))?;
        let socket_addr = listener.local_addr().unwrap();
        let db_conn = Self::get_db_pool(&config.database);
        let email_client = {
            let ec = &config.email_client;
            let sender = ec.sender().expect("Invalid sender email address.");
            let url = ec.url().expect("Invalid base url.");
            let timeout = ec.timeout();
            let auth_token = ec.auth_token.clone();
            EmailClient::new(url, sender, auth_token, timeout)
        };
        let base_url = AppBaseUrl(config.application.base_url.clone());

        // create the app runner
        let server = Self::get_server_runner(listener, db_conn, email_client, base_url)?;

        Ok(Self {
            server,
            socket_addr,
        })
    }

    fn get_server_runner(
        listener: TcpListener,
        db_pool: PgPool,
        email_client: EmailClient,
        base_url: AppBaseUrl,
    ) -> Result<Server> {
        let db_pool = Data::new(db_pool);
        let email_client = Data::new(email_client);
        let base_url = Data::new(base_url);
        let server = HttpServer::new(move || {
            actix_web::App::new()
                .wrap(TracingLogger::default())
                .service(routes::health_check)
                .service(routes::subscribe)
                .service(routes::confirm)
                .app_data(Data::clone(&db_pool))
                .app_data(Data::clone(&email_client))
                .app_data(Data::clone(&base_url))
        })
        .listen(listener)?
        .run();

        Ok(server)
    }

    pub fn addr(&self) -> SocketAddr {
        self.socket_addr
    }

    pub fn get_db_pool(config: &DatabaseSettings) -> PgPool {
        PgPool::connect_lazy_with(config.connect_options())
    }

    pub async fn run_until_stopped(self) -> Result<()> {
        self.server.await
    }
}

pub struct AppBaseUrl(pub String);
