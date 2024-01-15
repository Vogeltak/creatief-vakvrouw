use anyhow::Result;
use chrono::NaiveDate;
use reqwest::Url;
use serde::Deserialize;

use std::{env, str::FromStr};

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
        let (year, month) = (year.parse::<i32>()?, month.parse::<u32>()?);
        let mut week = month * 4 - 4;
        let mut events: Vec<Event> = vec![];
        let auth_cookie = env::var("LINDA_AUTH")?;
        let client = reqwest::Client::new();

        // Create a date that implements Ord so we can check if we should
        // stop fetching weeks from the l1nda yet.
        // We use the first day of the next month.
        let first_day_out_of_scope = {
            let (next_y, next_m) = match month {
                1..=11 => (year, month + 1),
                12 => (year + 1, 1),
                _ => return Err(anyhow::anyhow!("invalid month: {month}")),
            };
            NaiveDate::from_ymd_opt(next_y, next_m, 1)
        }
        .ok_or(anyhow::anyhow!("date is out-of-range"))?;

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
                    .filter(|d| {
                        d.date
                            .starts_with(format!("{}-{:02}", year, month).as_str())
                    })
                    .flat_map(|d| d.events)
                    .filter(|e| e.person == self.employee),
            );

            // Stop querying if we are about to start in a week that is past
            // the month we were processing.
            let week_end_date = NaiveDate::from_str(&v.end_date)?;
            if week_end_date >= first_day_out_of_scope {
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
