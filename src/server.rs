use crate::anita::{Anita, AnitaForm};
use crate::db;
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
    clients: Vec<factuur::Client>,
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

#[derive(Template)]
#[template(path = "btw.html")]
struct BtwTemplate {
    quarters: Vec<(Quarter, Btw)>,
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
    db: SqlitePool,
}

pub async fn run() -> Result<()> {
    let db_pool = SqlitePool::connect("/data/facturen.db?mode=rwc").await?;
    sqlx::migrate!().run(&db_pool).await?;

    let state = AppState { db: db_pool };

    let router = Router::new()
        .route("/", get(root_get))
        .route("/anita", get(anita_get))
        .route("/anita", post(anita_post))
        .route("/factuur", get(factuur_get))
        .route("/factuur", post(factuur_post))
        .route("/alle", get(history_get))
        .route("/btw", get(btw_get))
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

    let mut conn = state.db.acquire().await.unwrap();
    let anita = match db::get_client(&mut conn, "V.O.F. De Nieuwe Anita").await {
        Ok(a) => a,
        Err(_) => None,
    };

    FactuurTemplate {
        client: anita,
        items,
    }
}

async fn factuur_get(
    State(state): State<AppState>,
    Query(params): Query<FactuurParams>,
) -> FactuurTemplate {
    let client = match params.client {
        None => None,
        Some(key) => {
            let mut conn = state.db.acquire().await.unwrap();
            match db::get_client(&mut conn, &key).await {
                Ok(client) => client,
                Err(_) => None,
            }
        }
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
    match db::add_invoice(&mut conn, &factuur).await {
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
    let mut invoices = match db::get_invoices(&mut conn).await {
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

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
struct Quarter {
    year: i32,
    quarter: u32,
}

impl std::fmt::Display for Quarter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let quarters = HashMap::from([
            (1, "jan–maa"),
            (2, "apr–jun"),
            (3, "jul–sep"),
            (4, "okt–dec"),
        ]);

        let display_quarter = match quarters.get(&self.quarter) {
            Some(q) => q,
            None => "ooit",
        };

        write!(f, "{}Q{} ({})", self.year, self.quarter, display_quarter)
    }
}

#[derive(Debug, Clone)]
struct Btw {
    omzet: f64,
    btw: f64,
    invoices: Vec<Factuur>,
}

async fn btw_get(State(state): State<AppState>) -> BtwTemplate {
    let mut conn = state.db.acquire().await.unwrap();
    let mut invoices = match db::get_invoices(&mut conn).await {
        Ok(invoices) => invoices,
        Err(err) => {
            println!("Failed to fetch invoices from DB: {:?}", err);
            vec![]
        }
    };

    invoices.sort_by_key(|i| i.nummer);
    invoices.reverse();

    let mut grouped_invoices = HashMap::new();

    // Group by quarter
    for i in invoices {
        let q = Quarter {
            year: i.date.year(),
            // Default division behavior on unsigned integers is to floor
            quarter: i.date.month() / 4 + 1,
        };
        grouped_invoices.entry(q).or_insert(vec![]).push(i);
    }

    let mut grouped_invoices: Vec<(Quarter, Btw)> = grouped_invoices
        .into_iter()
        .map(|(q, invoices)| {
            let omzet = invoices.iter().map(|i| i.subtotal).sum();
            let btw = invoices.iter().map(|i| i.btw).sum();
            (
                q,
                Btw {
                    omzet,
                    btw,
                    invoices,
                },
            )
        })
        .collect();

    grouped_invoices.sort_by(|(ym1, _), (ym2, _)| ym1.cmp(ym2));
    grouped_invoices.reverse();

    BtwTemplate {
        quarters: grouped_invoices,
    }
}
