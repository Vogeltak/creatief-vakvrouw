use anyhow::Result;
use axum::{response::Html, routing::get, Router, Server};

pub async fn run() -> Result<()> {
    let router = Router::new()
        .route("/", get(root_get))
        .route("/anita", get(anita_get));

    let server = Server::bind(&"0.0.0.0:1728".parse()?).serve(router.into_make_service());
    let addr = server.local_addr();
    println!("Listening on {addr}");

    server.await?;

    Ok(())
}

pub async fn root_get() -> Html<&'static str> {
    Html(include_str!("../templates/index.html"))
}

pub async fn anita_get() -> Html<&'static str> {
    Html(include_str!("../templates/anita.html"))
}