use crate::event;

use askama::Template;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;

use std::error::Error;
use std::fmt::{Display, Formatter};
use std::io::{self, Write};
use std::iter::zip;
use std::path::{Path, PathBuf};
use std::process::Command;

const BTW_RATE: f64 = 0.21;

#[derive(Clone, Debug, Deserialize)]
pub struct FactuurForm {
    pub factuur_nummer: usize,
    pub client_name: String,
    pub client_address: String,
    pub client_zip: String,
    #[serde(rename = "task")]
    pub tasks: Vec<String>,
    #[serde(rename = "price")]
    pub prices: Vec<String>,
}

#[derive(Template)]
#[template(path = "invoice/details.yml")]
pub struct FactuurTemplate<'a> {
    pub factuur: &'a Factuur,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Factuur {
    pub nummer: usize,
    pub client: Client,
    pub work_items: Vec<WorkItem>,
    pub subtotal: f64,
    pub btw: f64,
    pub total: f64,
    pub date: DateTime<Utc>,
}

impl From<FactuurForm> for Factuur {
    fn from(value: FactuurForm) -> Self {
        let work_items: Vec<WorkItem> = zip(value.tasks, value.prices)
            .filter(|(desc, euro)| !desc.is_empty() && !euro.is_empty())
            .filter_map(|(desc, euro)| match euro.parse::<f64>() {
                Err(_) => None,
                Ok(euro) => Some((desc, euro)),
            })
            .map(|(desc, euro)| WorkItem { desc, euro })
            .collect();

        let subtotal = work_items.iter().map(|i| i.euro).sum::<f64>();
        let btw = BTW_RATE * subtotal;
        let total = subtotal + btw;

        Factuur {
            nummer: value.factuur_nummer,
            client: Client {
                name: value.client_name,
                address: value.client_address,
                zip: value.client_zip,
            },
            work_items,
            subtotal,
            btw,
            total,
            date: chrono::offset::Utc::now(),
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
    pub euro: f64,
}

impl TryFrom<event::Event> for WorkItem {
    type Error = FactuurError;

    fn try_from(e: event::Event) -> Result<Self, FactuurError> {
        let desc = format!("{} {} ({})", e.event_type, e.date, e.start_to_end);
        let start = DateTime::parse_from_rfc3339(&format!("{}+01:00", e.start)).map_err(|err| {
            FactuurError {
                kind: FactuurErrorKind::ParseDate(err),
            }
        })?;
        let end = DateTime::parse_from_rfc3339(&format!("{}+01:00", e.end)).map_err(|err| {
            FactuurError {
                kind: FactuurErrorKind::ParseDate(err),
            }
        })?;
        let hours = (end - start).num_minutes() as f64 / 60.0;
        let tarief = 22.0;
        let total = hours * tarief;

        Ok(Self { desc, euro: total })
    }
}

const TEX_TEMPLATE: &[u8] = include_bytes!("../templates/invoice/template.tex");

impl Factuur {
    pub fn generate_pdf(&self) -> Result<FactuurFile, FactuurError> {
        // Generate details from YAML template
        let factuur_details = FactuurTemplate { factuur: self }.render().unwrap();
        let mut details_file = NamedTempFile::new().map_err(|err| FactuurError {
            kind: FactuurErrorKind::ReadFile(err),
        })?;
        details_file
            .write_all(factuur_details.as_bytes())
            .map_err(|err| FactuurError {
                kind: FactuurErrorKind::ReadFile(err),
            })?;

        // Write the latex template to a temporary file so we can use it in our command
        let mut tex_file = NamedTempFile::new().map_err(|err| FactuurError {
            kind: FactuurErrorKind::ReadFile(err),
        })?;
        tex_file
            .write_all(TEX_TEMPLATE)
            .map_err(|err| FactuurError {
                kind: FactuurErrorKind::ReadFile(err),
            })?;

        // Call pandoc with details.yml and template.tex and save to /tmp
        let output_path = PathBuf::from(format!(
            "/tmp/Factuur {} {}.pdf",
            self.client.name, self.nummer
        ));
        let output = Command::new("pandoc")
            .arg(details_file.path())
            .arg("-o")
            .arg(format!("{}", output_path.to_string_lossy()))
            .arg(format!("--template={}", tex_file.path().to_string_lossy()))
            .arg("--pdf-engine=xelatex")
            .output()
            .map_err(|err| FactuurError {
                kind: FactuurErrorKind::PandocCommand(err),
            })?;

        match output.status.success() {
            true => Ok(FactuurFile(output_path)),
            false => Err(FactuurError {
                kind: FactuurErrorKind::PandocCommand(io::Error::new(
                    io::ErrorKind::Other,
                    String::from_utf8_lossy(&output.stderr).to_string(),
                )),
            }),
        }
    }
}

#[derive(Debug)]
pub struct FactuurError {
    pub kind: FactuurErrorKind,
}

impl Display for FactuurError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "error creating factuur: {:?}", self.kind)
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

impl AsRef<Path> for FactuurFile {
    #[inline]
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

impl Drop for FactuurFile {
    fn drop(&mut self) {
        _ = std::fs::remove_file(&self.0);
    }
}
