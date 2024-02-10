pub mod anita;
pub mod cli;
pub mod db;
pub mod event;
pub mod factuur;
pub mod routes;
pub mod server;

#[derive(Debug, PartialEq, Eq)]
pub enum Page {
    Dashboard,
    Facturen,
    Btw,
    Anita,
    Factuur,
    Auth,
}
