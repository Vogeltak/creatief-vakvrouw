use std::collections::HashMap;
use std::env;
use std::sync::Arc;

use crate::{db, routes};
use crate::{factuur, Page};

use anyhow::Result;
use askama::Template;

use axum::{
    extract::State,
    routing::{get, post},
    Router, Server,
};
use axum_login::axum_sessions::async_session::MemoryStore as SessionMemoryStore;
use axum_login::axum_sessions::SessionLayer;
use axum_login::memory_store::MemoryStore as AuthMemoryStore;
use axum_login::secrecy::SecretVec;
use axum_login::{AuthLayer, AuthUser, RequireAuthorizationLayer};

use chrono::prelude::*;
use rand::Rng;

use sqlx::sqlite::SqlitePool;
use tokio::sync::RwLock;

pub mod filters {
    use chrono::{TimeZone, Utc};

    use crate::factuur::Factuur;

    pub fn date<T: std::fmt::Display>(s: T) -> ::askama::Result<String> {
        // Entries ingested from Rust are explicitly set to RFC 3339 format
        match Utc.datetime_from_str(&s.to_string(), "%Y-%m-%d %H:%M:%S.%f UTC") {
            Ok(d) => Ok(format!("{}", d.format("%Y-%m-%d"))),
            Err(_) => match Utc.datetime_from_str(&s.to_string(), "%Y-%m-%d %H:%M:%S UTC") {
                // Fallback to default SQLite datetime() output
                Ok(d) => Ok(format!("{}", d.format("%Y-%m-%d"))),
                // Otherwise, just parrot back the original format
                Err(_) => Ok(format!("{s}")),
            },
        }
    }

    pub fn sum_invoices(invoices: &[Factuur]) -> ::askama::Result<f64> {
        Ok(invoices.iter().map(|i| i.subtotal).sum())
    }
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub user: User,
}

#[derive(Debug, Clone)]
pub struct User {
    pub id: usize,
    pub password_hash: String,
}

impl User {
    fn new() -> Result<Self> {
        let secret = env::var("USER_SECRET")?;
        Ok(User {
            id: 1,
            password_hash: secret,
        })
    }
}

impl AuthUser<usize> for User {
    fn get_id(&self) -> usize {
        self.id
    }

    fn get_password_hash(&self) -> SecretVec<u8> {
        SecretVec::new(self.password_hash.clone().into())
    }
}

pub async fn run() -> Result<()> {
    let db_pool = SqlitePool::connect("/data/facturen.db?mode=rwc").await?;
    sqlx::migrate!().run(&db_pool).await?;

    let user = User::new()?;
    let state = AppState {
        db: db_pool,
        user: user.clone(),
    };

    let secret = rand::thread_rng().gen::<[u8; 64]>();

    let session_store = SessionMemoryStore::new();
    let session_layer = SessionLayer::new(session_store, &secret);

    let store = Arc::new(RwLock::new(HashMap::default()));

    store.write().await.insert(user.get_id(), user);

    let user_store = AuthMemoryStore::new(&store);
    let auth_layer = AuthLayer::new(user_store, &secret);

    let router = Router::new()
        .route("/", get(root_get))
        .route("/anita", get(routes::anita::get))
        .route("/anita", post(routes::anita::post))
        .route("/factuur", get(routes::factuur::get))
        .route("/factuur", post(routes::factuur::post))
        .route("/download", get(routes::factuur::download))
        .route("/facturen", get(routes::report::history_get))
        .route("/btw", get(routes::report::btw_get))
        .route_layer(RequireAuthorizationLayer::<usize, User>::login_or_redirect(
            Arc::new("/login".into()),
            None,
        ))
        .route("/login", get(routes::auth::login_get))
        .route("/login", post(routes::auth::login_post))
        .layer(auth_layer)
        .layer(session_layer)
        .with_state(state);

    let server = Server::bind(&"0.0.0.0:1728".parse()?).serve(router.into_make_service());
    let addr = server.local_addr();
    println!("Listening on {addr}");

    server.await?;

    Ok(())
}

#[derive(Template)]
#[template(path = "index.html")]
struct PortaalTemplate {
    page: Page,
    clients: Vec<factuur::Client>,
    omzet: f64,
    laatste: Option<factuur::Factuur>,
}

async fn root_get(State(state): State<AppState>) -> PortaalTemplate {
    let mut conn = state.db.acquire().await.unwrap();
    let clients = match db::get_all_clients(&mut conn).await {
        Ok(clients) => clients,
        Err(_) => vec![],
    };

    let mut invoices = match db::get_invoices(&mut conn).await {
        Ok(invoices) => invoices,
        Err(err) => {
            println!("Failed to fetch invoices for dashboard: {err}");
            vec![]
        }
    };

    let now = Utc::now();
    let omzet = invoices
        .iter()
        .filter(|i| i.date.year() == now.year())
        .map(|i| i.subtotal)
        .sum();

    invoices.sort_by_key(|i| i.nummer);
    invoices.reverse();
    let laatste = invoices.first().cloned();

    PortaalTemplate {
        page: Page::Dashboard,
        clients,
        omzet,
        laatste,
    }
}
