use crate::factuur::{self};
use crate::{db, routes};

use anyhow::Result;
use askama::Template;

use axum::{
    extract::State,
    routing::{get, post},
    Router, Server,
};

use sqlx::sqlite::SqlitePool;

#[derive(Template)]
#[template(path = "index.html")]
struct PortaalTemplate {
    clients: Vec<factuur::Client>,
}

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
}

pub async fn run() -> Result<()> {
    let db_pool = SqlitePool::connect("/data/facturen.db?mode=rwc").await?;
    sqlx::migrate!().run(&db_pool).await?;

    let state = AppState { db: db_pool };

    let router = Router::new()
        .route("/", get(root_get))
        .route("/anita", get(routes::anita::get))
        .route("/anita", post(routes::anita::post))
        .route("/factuur", get(routes::factuur::get))
        .route("/factuur", post(routes::factuur::post))
        .route("/alle", get(routes::report::history_get))
        .route("/btw", get(routes::report::btw_get))
        .with_state(state);

    let server = Server::bind(&"0.0.0.0:1728".parse()?).serve(router.into_make_service());
    let addr = server.local_addr();
    println!("Listening on {addr}");

    server.await?;

    Ok(())
}

async fn root_get(State(state): State<AppState>) -> PortaalTemplate {
    let mut conn = state.db.acquire().await.unwrap();
    let clients = match db::get_all_clients(&mut conn).await {
        Ok(clients) => clients,
        Err(_) => vec![],
    };

    PortaalTemplate { clients }
}
