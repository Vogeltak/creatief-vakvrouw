use chrono::DateTime;
use clap::Parser;
use reqwest::Url;
use serde_json::Value;

#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Cli {
    #[arg(short, long)]
    month: String,
    #[arg(short, long, default_value = "Noemi")]
    name: String,
}

struct Event {
    person: String,
    event_type: String,
    date: String,
    start_to_end: String,
    start: String,
    end: String,
}

impl From::<&Value> for Event {
    fn from(value: &Value) -> Self {
        Event {
            person: value["event_who_profile_call_name"].as_str().unwrap().to_owned(),
            event_type: value["event_type"].as_str().unwrap().to_owned(),
            date: value["event_date"].as_str().unwrap().to_owned(),
            start_to_end: value["event_start_end_time"].as_str().unwrap().to_owned(),
            start: value["event_starts_at"].as_str().unwrap().to_owned(),
            end: value["event_ends_at"].as_str().unwrap().to_owned(),
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // TODO: extract year and month from input, and iterate over weeks
    // until we've found all events belonging to that month

    let (year, month) = cli.month.split_once('-').unwrap();
    let mut week = month.parse::<u32>().unwrap() * 4 - 4;

    let mut events: Vec<Event> = vec![];

    loop {
        let url = format!(
            "https://denieuweanita.l1nda.nl/week/{}/{}?xhr=true",
            year,
            week
        )
        .parse::<Url>()
        .unwrap();
        let cookie =
            "csrftoken=wXue6RgLvFb39U4rfMLuVEEpw16WOiR1;sessionid=ugb1pibp3zfvj6lplz0pvnjn55p41ixp";

        let client = reqwest::blocking::Client::new();
        let res = client.get(url).header("Cookie", cookie).send()?;

        let body = res.text()?;
        let v: Value = serde_json::from_str(&body)?;

        println!("Got data from {} to {}", v["start_date"], v["end_date"]);

        events.extend(v["scheduled_events"]
            .as_array()
            .unwrap()
            .get(0)
            .unwrap()
            .get("layer_days")
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .filter(|day| day.get("day_key").unwrap().as_str().unwrap().starts_with(&cli.month))
            .flat_map(|day| day.get("day_events").unwrap().as_array().unwrap().iter())
            .filter(|&event| {
                event
                    .get("event_who_profile_call_name")
                    .unwrap()
                    .as_str()
                    .unwrap()
                    == cli.name
            })
            .map(Event::from)
        );

        // Break out if we are currently starting in a week that is past the
        // month we were processing.
        let start_month = v["start_date"].as_str().unwrap().split('-').take(2).last().unwrap();
        let end_month = v["end_date"].as_str().unwrap().split('-').take(2).last().unwrap();
        if start_month > month  || end_month > month {
            break;
        }

        week += 1;
    }

    for e in &events {
        println!(
            "{} works at {} from {}",
            e.person,
            e.date,
            e.start_to_end
        );
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
        events
            .last()
            .unwrap()
            .date
    );
    println!("service:");
    for e in events {
        let desc = format!(
            "{} {} ({})",
            e.event_type,
            e.date,
            e.start_to_end
        );
        let start = DateTime::parse_from_rfc3339(&format!(
            "{}+01:00",
            e.start
        ))
        .unwrap();
        let end = DateTime::parse_from_rfc3339(&format!(
            "{}+01:00",
            e.end
        ))
        .unwrap();
        let hours = (end - start).num_minutes() as f64 / 60.0;
        let tarief = 18.0;
        let total = hours * tarief;
        println!("- description: {}", desc);
        println!("  price: {}", total);
    }
}
