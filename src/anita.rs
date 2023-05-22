use anyhow::Result;
use reqwest::Url;
use serde::Deserialize;

use std::env;

use crate::event::{Event, Week};

#[derive(Clone, Debug)]
pub struct Anita {
    employee: String,
}

impl Anita {
    pub fn new(employee: String) -> Self {
        Anita { employee }
    }

    pub async fn get_events_from_month(&self, month: String, year: String) -> Result<Vec<Event>> {
        let mut week = month.parse::<u32>()? * 4 - 4;
        let mut events: Vec<Event> = vec![];
        let auth_cookie = env::var("LINDA_AUTH")?;
        let client = reqwest::Client::new();

        loop {
            let url = format!(
                "https://denieuweanita.l1nda.nl/week/{}/{}?xhr=true",
                year, week
            )
            .parse::<Url>()?;

            let res = client
                .get(url)
                .header("Cookie", &auth_cookie)
                .send()
                .await?;

            let body = res.text().await?;
            let v: Week = serde_json::from_str(&body)?;

            println!("Got data from {} to {}", v.start_date, v.end_date);

            events.extend(
                v.schedule
                    .iter()
                    .flat_map(|r| r.days.clone())
                    .filter(|d| d.date.starts_with(format!("{}-{}", year, month).as_str()))
                    .flat_map(|d| d.events)
                    .filter(|e| e.person == self.employee),
            );

            // Break out if we are currently starting in a week that is past the
            // month we were processing.
            let start_month = v.start_date.split('-').take(2).last().unwrap();
            let end_month = v.end_date.split('-').take(2).last().unwrap();
            if start_month > month.as_str() || end_month > month.as_str() {
                break;
            }

            week += 1;
        }

        Ok(events)
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct AnitaForm {
    pub maand: Month,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Month(pub String);

impl TryFrom<String> for Month {
    type Error = anyhow::Error;

    /// Should be of form YYYY-MM
    fn try_from(s: String) -> Result<Self> {
        match s.split_once('-') {
            Some(_) => {
                // TODO: Verify that year and month are within the allowed
                // numerical range.
                Ok(Self(s))
            }
            None => anyhow::bail!("invalid month notation: should be of form YYYY-MM"),
        }
    }
}
