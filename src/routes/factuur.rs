use std::{fmt::Display, str::FromStr};

use askama::Template;
use askama_axum::IntoResponse;
use axum::{
    extract::{Query, State},
    response::Redirect,
};
use axum_extra::extract::Form;
use reqwest::{header, StatusCode};
use serde::{Deserialize, Deserializer};

use crate::{
    db::{self, SoftDeleteAction},
    factuur::{self, Factuur, FactuurForm},
    server::AppState,
    Page,
};

#[derive(Template)]
#[template(path = "factuur.html")]
pub struct FactuurTemplate {
    pub page: Page,
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
        page: Page::Factuur,
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

    let pdf = match tokio::fs::read(&factuur_file).await {
        Ok(pdf) => pdf,
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

    // Persist invoice details to the database
    let mut conn = state.db.acquire().await.unwrap();
    match db::add_invoice(&mut conn, &factuur, &pdf).await {
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

    Ok(Redirect::to(
        format!("/facturen?n={}#{}", factuur.nummer, factuur.nummer).as_str(),
    ))
}

#[derive(Debug, Deserialize)]
pub struct FactuurActionParams {
    factuur: usize,
}

pub async fn download(
    State(state): State<AppState>,
    Query(params): Query<FactuurActionParams>,
) -> impl IntoResponse {
    let mut conn = state.db.acquire().await.unwrap();
    let (name, pdf) = match db::get_pdf(&mut conn, params.factuur as u32).await {
        Ok((n, p)) => (n, p),
        Err(err) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!(
                    "Hey, er gings iets mis bij het ophalen van de factuur uit de database. \
                    Laat dit even zien aan Max:\n\n {}",
                    err
                ),
            ))
        }
    };

    let headers = [
        (header::CONTENT_TYPE, "application/pdf".to_owned()),
        (
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", name),
        ),
    ];

    Ok((headers, pdf))
}

pub async fn delete(
    State(state): State<AppState>,
    Query(params): Query<FactuurActionParams>,
) -> impl IntoResponse {
    let mut conn = state.db.acquire().await.unwrap();
    match db::soft_delete_invoice(&mut conn, params.factuur as u32, SoftDeleteAction::Delete).await
    {
        Ok(_) => Ok(Redirect::to("/facturen")),
        Err(err) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!(
                    "Hey, er ging iets mis bij het verwijderen van de factuur uit de database. \
                    Laat dit even zien aan Max:\n\n {}",
                    err
                ),
            ))
        }
    }
}

pub async fn restore(
    State(state): State<AppState>,
    Query(params): Query<FactuurActionParams>,
) -> impl IntoResponse {
    let mut conn = state.db.acquire().await.unwrap();
    match db::soft_delete_invoice(&mut conn, params.factuur as u32, SoftDeleteAction::Restore).await
    {
        Ok(_) => Ok(Redirect::to(
            format!("/facturen?n={}#{}", params.factuur, params.factuur).as_str(),
        )),
        Err(err) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!(
                    "Hey, er ging iets mis bij het herstellen van de factuur uit de database. \
                    Laat dit even zien aan Max:\n\n {}",
                    err
                ),
            ))
        }
    }
}
