use crate::factuur::{Factuur, FactuurForm};

use anyhow::Result;
use askama::Template;
use axum::{http::{header, StatusCode}, response::IntoResponse, routing::{get, post}, Router, Server, body::StreamBody};
use axum_extra::extract::Form;
use tokio_util::io::ReaderStream;

#[derive(Template)]
#[template(path = "index.html")]
struct PortaalTemplate {}

#[derive(Template)]
#[template(path = "anita.html")]
struct AnitaTemplate<'a> {
    name: &'a str,
}

#[derive(Template)]
#[template(path = "factuur.html")]
struct FactuurTemplate {}

pub async fn run() -> Result<()> {
    let router = Router::new()
        .route("/", get(root_get))
        .route("/anita", get(anita_get))
        .route("/factuur", get(factuur_get))
        .route("/factuur", post(factuur_post));

    let server = Server::bind(&"0.0.0.0:1728".parse()?).serve(router.into_make_service());
    let addr = server.local_addr();
    println!("Listening on {addr}");

    server.await?;

    Ok(())
}

async fn root_get() -> PortaalTemplate {
    PortaalTemplate {}
}

async fn anita_get() -> AnitaTemplate<'static> {
    AnitaTemplate { name: "Noemi" }
}

async fn factuur_get() -> FactuurTemplate {
    FactuurTemplate {}
}

async fn factuur_post(Form(factuur_form): Form<FactuurForm>) -> impl IntoResponse {
    // TODO: Generate invoice
    let factuur = Factuur::from(factuur_form);
    println!("[Factuur {}] Number of work items: {}", factuur.nummer, factuur.work_items.len());

    let file = match tokio::fs::File::open("/tmp/test.pdf").await {
        Ok(file) => file,
        Err(err) => return Err((StatusCode::NOT_FOUND, format!("Failed to generate invoice: {}", err))),
    };

    // Convert the `AsyncRead` into a `Stream`
    let stream = ReaderStream::new(file);
    // Convert the `Stream` into an `axum::body::HttpBody`
    let body = StreamBody::new(stream);

    let headers = [
        (header::CONTENT_TYPE, "application/pdf"),
        (
            header::CONTENT_DISPOSITION,
            "attachment; filename=\"test.pdf\"",
        )
    ];

    Ok((headers, body))
}