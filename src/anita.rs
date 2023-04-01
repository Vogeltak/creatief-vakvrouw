use anyhow::Result;
use reqwest::Url;

use std::env;

use crate::event::{Week, Event};

#[derive(Clone, Debug)]
pub struct Anita {
    employee: String,
}

impl Anita {
    pub fn new(employee: String) -> Self {
        Anita {
            employee
        }
    }

    pub fn get_events_from_month(&self, month: String, year: String) -> Result<Vec<Event>> {
        let mut week = month.parse::<u32>()? * 4 - 4;
        let mut events: Vec<Event> = vec![];
        let auth_cookie = env::var("LINDA_AUTH")?;
        let client = reqwest::blocking::Client::new();

        loop {
            let url = format!(
                "https://denieuweanita.l1nda.nl/week/{}/{}?xhr=true",
                year, week
            )
            .parse::<Url>()?;

            let res = client.get(url).header("Cookie", &auth_cookie).send()?;

            let body = res.text()?;
            let v: Week = serde_json::from_str(&body)?;

            println!("Got data from {} to {}", v.start_date, v.end_date);

            events.extend(v.schedule.iter()
                .flat_map(|r| r.days.clone())
                .filter(|d| d.date.starts_with(format!("{}-{}", year, month).as_str()))
                .flat_map(|d| d.events)
                .filter(|e| e.person == self.employee)
            );

            // Break out if we are currently starting in a week that is past the
            // month we were processing.
            let start_month = v.start_date
                .split('-')
                .take(2)
                .last()
                .unwrap();
            let end_month = v.end_date
                .split('-')
                .take(2)
                .last()
                .unwrap();
            if start_month > month.as_str() || end_month > month.as_str() {
                break;
            }

            week += 1;
        }

        Ok(events)
    }
}