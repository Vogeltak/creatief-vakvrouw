use crate::event;

use chrono::DateTime;
use serde::{Deserialize, Serialize};

use std::error::Error;
use std::fmt::{Display, Formatter};
use std::io;
use std::iter::zip;
use std::path::PathBuf;

#[derive(Clone, Debug, Deserialize)]
pub struct FactuurForm {
    pub factuur_nummer: usize,
    pub client_name: String,
    pub client_address: String,
    pub client_zip: String,
    #[serde(rename = "task")]
    pub tasks: Vec<String>,
    #[serde(rename = "price")]
    pub prices: Vec<f32>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Factuur {
    pub nummer: usize,
    pub client: Client,
    pub work_items: Vec<WorkItem>,
}

impl From<FactuurForm> for Factuur {
    fn from(value: FactuurForm) -> Self {
        Factuur {
            nummer: value.factuur_nummer,
            client: Client {
                name: value.client_name,
                address: value.client_address,
                zip: value.client_zip,
            },
            work_items: zip(value.tasks, value.prices)
                .map(|(desc, euro)| WorkItem { desc, euro })
                .collect(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Client {
    pub name: String,
    pub address: String,
    pub zip: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WorkItem {
    pub desc: String,
    pub euro: f32,
}

impl From<event::Event> for WorkItem {
    fn from(e: event::Event) -> Self {
        let desc = format!("{} {} ({})", e.event_type, e.date, e.start_to_end);
        let start = DateTime::parse_from_rfc3339(&format!("{}+01:00", e.start)).unwrap();
        let end = DateTime::parse_from_rfc3339(&format!("{}+01:00", e.end)).unwrap();
        let hours = (end - start).num_minutes() as f32 / 60.0;
        let tarief = 18.0;
        let total = hours * tarief;

        Self { desc, euro: total }
    }
}

impl Factuur {
    pub fn generate_pdf() -> Result<FactuurFile, FactuurError> {
        Ok(FactuurFile(PathBuf::new()))
    }
}

#[derive(Debug)]
pub struct FactuurError {
    pub kind: FactuurErrorKind,
}

impl Display for FactuurError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "error creating factuur")
    }
}

impl Error for FactuurError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self.kind {
            FactuurErrorKind::ParseDate(err) => Some(err),
            FactuurErrorKind::PandocCommand(err) => Some(err),
            FactuurErrorKind::ReadFile(err) => Some(err),
        }
    }
}

#[derive(Debug)]
pub enum FactuurErrorKind {
    ParseDate(chrono::format::ParseError),
    PandocCommand(io::Error),
    ReadFile(io::Error),
}

/// Represents the invoice as a temporary PDF file on disk. Should be
/// deleted from disk on Drop to prevent disk space from filling up.
pub struct FactuurFile(PathBuf);

impl Drop for FactuurFile {
    fn drop(&mut self) {
        std::fs::remove_file(&self.0);
    }
}
