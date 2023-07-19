use crate::anita::{Anita, AnitaForm};
use crate::db::{add_invoice_to_db, get_invoices};
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
use chrono::Datelike;
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

#[derive(Template)]
#[template(path = "history.html")]
struct HistoryTemplate {
    grouped_invoices: Vec<(YearMonth, Vec<Factuur>)>,
}

mod filters {
    use chrono::{TimeZone, Utc};

    pub fn date<T: std::fmt::Display>(s: T) -> ::askama::Result<String> {
        let parsed = Utc
            .datetime_from_str(&s.to_string(), "%Y-%m-%d %H:%M:%S UTC")
            .map_err(|err| ::askama::Error::Custom(Box::new(err)))?;
        Ok(format!("{}", parsed.format("%Y-%m-%d")))
    }
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
        .route("/history", get(history_get))
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

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
struct YearMonth {
    year: i32,
    month: u32,
}

impl std::fmt::Display for YearMonth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let months = HashMap::from([
            (1, "Januari"),
            (2, "Februari"),
            (3, "Maart"),
            (4, "April"),
            (5, "Mei"),
            (6, "Juni"),
            (7, "Juli"),
            (8, "Augustus"),
            (9, "September"),
            (10, "Oktober"),
            (11, "November"),
            (12, "December"),
        ]);

        let display_month = match months.get(&self.month) {
            Some(m) => m,
            None => "Ooit",
        };

        write!(f, "{} {}", display_month.to_lowercase(), self.year)
    }
}

async fn history_get(State(state): State<AppState>) -> HistoryTemplate {
    let mut conn = state.db.acquire().await.unwrap();
    let mut invoices = match get_invoices(&mut conn).await {
        Ok(invoices) => invoices,
        Err(err) => {
            println!("Failed to fetch invoices from DB: {:?}", err);
            vec![]
        }
    };

    invoices.sort_by_key(|i| i.nummer);
    invoices.reverse();

    let mut grouped_invoices = HashMap::new();

    // Group by month
    for i in invoices {
        let year_month = YearMonth {
            year: i.date.year(),
            month: i.date.month(),
        };
        grouped_invoices.entry(year_month).or_insert(vec![]).push(i);
    }

    let mut grouped_invoices: Vec<(YearMonth, Vec<Factuur>)> = grouped_invoices
        .into_iter()
        .map(|(ym, invoices)| (ym, invoices))
        .collect();

    grouped_invoices.sort_by(|(ym1, _), (ym2, _)| ym1.cmp(ym2));
    grouped_invoices.reverse();

    HistoryTemplate { grouped_invoices }
}
