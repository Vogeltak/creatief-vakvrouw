use chrono::DateTime;
use clap::Parser;
use reqwest::Url;

mod cli;
mod event;

use event::Event;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arg = cli::Cli::parse();

    let (year, month) = arg.month.split_once('-').unwrap();
    let mut week = month.parse::<u32>().unwrap() * 4 - 4;

    let mut events: Vec<Event> = vec![];

    loop {
        let url = format!(
            "https://denieuweanita.l1nda.nl/week/{}/{}?xhr=true",
            year, week
        )
        .parse::<Url>()
        .unwrap();
        let cookie =
            "csrftoken=wXue6RgLvFb39U4rfMLuVEEpw16WOiR1;sessionid=ugb1pibp3zfvj6lplz0pvnjn55p41ixp";

        let client = reqwest::blocking::Client::new();
        let res = client.get(url).header("Cookie", cookie).send()?;

        let body = res.text()?;
        let v: event::Week = serde_json::from_str(&body)?;

        println!("Got data from {} to {}", v.start_date, v.end_date);

        events.extend(v.schedule.iter()
            .flat_map(|r| r.days.clone())
            .filter(|d| d.date.starts_with(&arg.month))
            .flat_map(|d| d.events)
            .filter(|e| e.person == arg.name)
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
        if start_month > month || end_month > month {
            break;
        }

        week += 1;
    }

    for e in &events {
        println!("{} works at {} from {}", e.person, e.date, e.start_to_end);
    }

    if events.is_empty() {
        return Err("No relevant events".into());
    }

    println!();
    invoice(&events);

    Ok(())
}

fn invoice(events: &[Event]) {
    println!(
        "Factuur De Nieuwe Anita ({} t/m {})",
        events[0].date,
        events.last().unwrap().date
    );
    println!("service:");
    for e in events {
        let desc = format!("{} {} ({})", e.event_type, e.date, e.start_to_end);
        let start = DateTime::parse_from_rfc3339(&format!("{}+01:00", e.start)).unwrap();
        let end = DateTime::parse_from_rfc3339(&format!("{}+01:00", e.end)).unwrap();
        let hours = (end - start).num_minutes() as f64 / 60.0;
        let tarief = 18.0;
        let total = hours * tarief;
        println!("- description: {}", desc);
        println!("  price: {}", total);
    }
}
