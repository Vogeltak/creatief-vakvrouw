use crate::anita::{Anita, AnitaForm};
use crate::db::add_invoice_to_db;
use crate::factuur::{self, Factuur, FactuurForm};

use anyhow::Result;
use askama::Template;
use axum::extract::Query;
use axum::{
    body::StreamBody,
    extract::State,
    http::{header, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Router, Server,
};
use axum_extra::extract::Form;
use serde::{Deserialize, Deserializer};
use sqlx::sqlite::SqlitePool;
use tokio_util::io::ReaderStream;

use std::collections::HashMap;
use std::fmt::Display;
use std::str::FromStr;

#[derive(Template)]
#[template(path = "index.html")]
struct PortaalTemplate {
    clients: HashMap<String, factuur::Client>,
}

#[derive(Template)]
#[template(path = "anita.html")]
struct AnitaTemplate {}

#[derive(Template)]
#[template(path = "factuur.html")]
struct FactuurTemplate {
    client: Option<factuur::Client>,
    items: Vec<factuur::WorkItem>,
}

#[derive(Debug, Clone)]
struct AppState {
    clients: HashMap<String, factuur::Client>,
    db: SqlitePool,
}

pub async fn run() -> Result<()> {
    let clients = match serde_json::from_str(include_str!("../clients.json")) {
        Ok(c) => c,
        Err(_) => anyhow::bail!("invalidly formatted clients.json"),
    };

    let db_pool = SqlitePool::connect("facturen.db").await?;
    sqlx::migrate!().run(&db_pool).await?;

    let state = AppState {
        clients,
        db: db_pool,
    };

    let router = Router::new()
        .route("/", get(root_get))
        .route("/anita", get(anita_get))
        .route("/anita", post(anita_post))
        .route("/factuur", get(factuur_get))
        .route("/factuur", post(factuur_post))
        .with_state(state);

    let server = Server::bind(&"0.0.0.0:1728".parse()?).serve(router.into_make_service());
    let addr = server.local_addr();
    println!("Listening on {addr}");

    server.await?;

    Ok(())
}

async fn root_get(State(state): State<AppState>) -> PortaalTemplate {
    PortaalTemplate {
        clients: state.clients,
    }
}

async fn anita_get() -> AnitaTemplate {
    AnitaTemplate {}
}

async fn anita_post(
    State(state): State<AppState>,
    Form(anita_form): Form<AnitaForm>,
) -> FactuurTemplate {
    let (year, month) = anita_form.maand.0.split_once('-').unwrap();
    let items = match Anita::new("Noemi".to_string())
        .get_events_from_month(month.to_owned(), year.to_owned())
        .await
    {
        Ok(events) => events
            .iter()
            .cloned()
            .filter_map(|e| factuur::WorkItem::try_from(e).ok())
            .collect::<Vec<factuur::WorkItem>>(),
        Err(err) => {
            println!("Failed to fetch data from L1NDA: {}", err);
            vec![]
        }
    };

    FactuurTemplate {
        client: state.clients.get("anita").cloned(),
        items,
    }
}

async fn factuur_get(
    State(state): State<AppState>,
    Query(params): Query<FactuurParams>,
) -> FactuurTemplate {
    let client = match params.client {
        None => None,
        Some(key) => state.clients.get(&key).cloned(),
    };

    FactuurTemplate {
        client,
        items: vec![],
    }
}

#[derive(Debug, Deserialize)]
struct FactuurParams {
    #[serde(default, deserialize_with = "empty_string_as_none")]
    client: Option<String>,
}

fn empty_string_as_none<'de, D, T>(de: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr,
    T::Err: Display,
{
    let opt = Option::<String>::deserialize(de)?;
    match opt.as_deref() {
        None | Some("") => Ok(None),
        Some(s) => FromStr::from_str(s)
            .map_err(serde::de::Error::custom)
            .map(Some),
    }
}

async fn factuur_post(
    State(state): State<AppState>,
    Form(factuur_form): Form<FactuurForm>,
) -> impl IntoResponse {
    let factuur = Factuur::from(factuur_form);

    // Persist invoice details to the database
    let mut conn = state.db.acquire().await.unwrap();
    match add_invoice_to_db(&mut conn, &factuur).await {
        Ok(_) => (),
        Err(err) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!(
                    "Hey, foutje tijdens het updaten van de database. \
                    Laat dit even zien aan Max:\n\n {}",
                    err
                ),
            ))
        }
    };

    let factuur_file = match factuur.generate_pdf() {
        Ok(f) => f,
        Err(err) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!(
                    "Hey er ging iets mis tijdens het genereren van de PDF. \
                    Laat dit even zien aan Max:\n\n {}",
                    err
                ),
            ))
        }
    };

    let file = match tokio::fs::File::open(&factuur_file).await {
        Ok(file) => file,
        Err(err) => {
            return Err((
                StatusCode::NOT_FOUND,
                format!(
                    "Hey er ging iets mis tijdens het genereren van de PDF. \
                    Laat dit even zien aan Max:\n\n {}",
                    err
                ),
            ))
        }
    };

    // Convert the `AsyncRead` into a `Stream`
    let stream = ReaderStream::new(file);
    // Convert the `Stream` into an `axum::body::HttpBody`
    let body = StreamBody::new(stream);

    let headers = [
        (header::CONTENT_TYPE, "application/pdf".to_owned()),
        (
            header::CONTENT_DISPOSITION,
            format!(
                "attachment; filename=\"Factuur {} {}.pdf\"",
                factuur.client.name, factuur.nummer
            ),
        ),
    ];

    Ok((headers, body))
}
