use axum::{response::Html, routing::get, Router, Server};

#[tokio::main]
async fn main() {
    let router = Router::new()
        .route("/", get(root_get))
        .route("/anita", get(anita_get));

    let server = Server::bind(&"0.0.0.0:1728".parse().unwrap()).serve(router.into_make_service());
    let addr = server.local_addr();
    println!("Listening on {addr}");

    server.await.unwrap();

    println!("Hello, world!");
}

async fn root_get() -> Html<&'static str> {
    Html(include_str!("index.html"))
}

async fn anita_get() -> Html<&'static str> {
    Html(include_str!("anita.html"))
}