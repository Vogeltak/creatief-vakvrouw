use std::{fmt::Display, str::FromStr};

use askama::Template;
use askama_axum::IntoResponse;
use axum::{
    body::StreamBody,
    extract::{Query, State},
};
use axum_extra::extract::Form;
use reqwest::{header, StatusCode};
use serde::{Deserialize, Deserializer};
use tokio_util::io::ReaderStream;

use crate::{
    db,
    factuur::{self, Factuur, FactuurForm},
    server::AppState,
};

#[derive(Template)]
#[template(path = "factuur.html")]
pub struct FactuurTemplate {
    pub client: Option<factuur::Client>,
    pub items: Vec<factuur::WorkItem>,
    pub most_recent_invoice_id: Option<usize>,
}

pub async fn get(
    State(state): State<AppState>,
    Query(params): Query<FactuurParams>,
) -> FactuurTemplate {
    let mut conn = state.db.acquire().await.unwrap();

    let client = match params.client {
        None => None,
        Some(key) => match db::get_client(&mut conn, &key).await {
            Ok(client) => client,
            Err(_) => None,
        },
    };

    let most_recent_invoice_id = match db::most_recent_invoice(&mut conn).await {
        Ok(i) => i,
        Err(_) => None,
    };

    FactuurTemplate {
        client,
        items: vec![],
        most_recent_invoice_id,
    }
}

#[derive(Debug, Deserialize)]
pub struct FactuurParams {
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

pub async fn post(
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
