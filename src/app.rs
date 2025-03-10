use crate::{auth::reject_anonymous_users, config::Settings, email_client::EmailClient, routes::*};
use actix_session::{
    storage::{RedisSessionStore, SessionStore},
    SessionMiddleware,
};
use actix_web::{
    cookie::Key, dev::Server, middleware::from_fn as mw_fn, web, web::Data, HttpServer,
};
use actix_web_flash_messages::{storage::CookieMessageStore, FlashMessagesFramework};
use core::net::SocketAddr;
use secrecy::{ExposeSecret, SecretString};
use sqlx::PgPool;
use std::net::TcpListener;
use tracing_actix_web::TracingLogger;

#[derive(Clone)]
pub struct HmacSecret(pub SecretString);

pub struct App {
    server: Server,
    socket_addr: SocketAddr,
}

impl App {
    pub async fn build(config: &Settings) -> anyhow::Result<Self> {
        // create the app dependencies
        let listener =
            TcpListener::bind((config.application.host.clone(), config.application.port))?;
        let socket_addr = listener.local_addr().unwrap();
        let db_conn = config.database.get_db_pool();
        let email_client = config.email_client.client();
        let base_url = AppBaseUrl(config.application.base_url.clone());
        let hmac_secret = config.application.hmac_secret.clone();
        let session_store = RedisSessionStore::new(config.redis_uri.expose_secret()).await?;

        // create the app runner
        let server = Self::get_server_runner(
            listener,
            db_conn,
            email_client,
            base_url,
            hmac_secret,
            session_store,
        )?;

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
        hmac_secret: SecretString,
        session_store: impl SessionStore + Send + Clone + 'static,
    ) -> anyhow::Result<Server> {
        let db_pool = Data::new(db_pool);
        let email_client = Data::new(email_client);
        let base_url = Data::new(base_url);
        let secret_key = Key::from(hmac_secret.expose_secret().as_bytes());
        let message_framework = {
            let store = CookieMessageStore::builder(secret_key.clone()).build();
            FlashMessagesFramework::builder(store).build()
        };
        let hmac_secret = Data::new(HmacSecret(hmac_secret));
        let server = HttpServer::new(move || {
            actix_web::App::new()
                .wrap(message_framework.clone())
                .wrap(SessionMiddleware::new(
                    session_store.clone(),
                    secret_key.clone(),
                ))
                .wrap(TracingLogger::default())
                .service(health_check)
                .service(subscribe)
                .service(confirm)
                .service(home)
                .service(login_form)
                .service(login)
                .service(
                    web::scope("/admin")
                        .wrap(mw_fn(reject_anonymous_users))
                        .service(admin_dashboard)
                        .service(newsletters_form)
                        .service(publish_newsletter)
                        .service(change_password)
                        .service(change_password_form)
                        .service(logout),
                )
                .app_data(Data::clone(&db_pool))
                .app_data(Data::clone(&email_client))
                .app_data(Data::clone(&base_url))
                .app_data(Data::clone(&hmac_secret))
        })
        .listen(listener)?
        .run();

        Ok(server)
    }

    pub fn addr(&self) -> SocketAddr {
        self.socket_addr
    }

    pub async fn run_until_stopped(self) -> anyhow::Result<()> {
        self.server.await?;
        Ok(())
    }
}

pub struct AppBaseUrl(pub String);
