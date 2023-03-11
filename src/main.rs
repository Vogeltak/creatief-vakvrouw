use std::{fs::{self, File}, io::Write};

use chrono::DateTime;
use clap::Parser;
use reqwest::Url;
use serde_json::Value;

#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Cli {
    #[arg(short, long)]
    week: u8,
    #[arg(short, long, default_value = "Noemi")]
    name: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let url = format!(
        "https://denieuweanita.l1nda.nl/week/2023/{}?xhr=true",
        cli.week
    )
    .parse::<Url>()
    .unwrap();
    let cookie =
        "csrftoken=wXue6RgLvFb39U4rfMLuVEEpw16WOiR1;sessionid=ugb1pibp3zfvj6lplz0pvnjn55p41ixp";

    let client = reqwest::blocking::Client::new();
    let res = client.get(url).header("Cookie", cookie).send()?;

    eprintln!("Headers: {:#?}", res.headers());
    let body = res.text()?;
    let v: Value = serde_json::from_str(&body)?;

    println!("Got data from {} to {}", v["start_date"], v["end_date"]);

    let events = v["scheduled_events"]
        .as_array()
        .unwrap()
        .get(0)
        .unwrap()
        .get("layer_days")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|day| day.get("day_events").unwrap().as_array().unwrap().iter())
        .filter(|&event| {
            event
                .get("event_who_profile_call_name")
                .unwrap()
                .as_str()
                .unwrap()
                == cli.name
        })
        .collect::<Vec<&Value>>();

    for e in &events {
        println!(
            "{} works at {} from {}",
            cli.name,
            e.get("event_date").unwrap().as_str().unwrap(),
            e.get("event_start_end_time").unwrap().as_str().unwrap()
        );
    }

    println!();
    invoice(&events);

    Ok(())
}

fn invoice(events: &[&Value]) {
    let mut template = fs::read_to_string("template-nieuwe-anita.md").unwrap();
    println!(
        "Factuur De Nieuwe Anita ({} t/m {})",
        events[0].get("event_date").unwrap().as_str().unwrap(),
        events
            .last()
            .unwrap()
            .get("event_date")
            .unwrap()
            .as_str()
            .unwrap()
    );
    template.push_str("Omschrijving | Uren | Uurtarief | Totaal\n");
    template.push_str("---|---|---|---\n");
    let mut subtotaal = 0.0;
    for e in events {
        let desc = format!(
            "{} {} ({})",
            e.get("event_type").unwrap().as_str().unwrap(),
            e.get("event_date").unwrap().as_str().unwrap(),
            e.get("event_start_end_time").unwrap().as_str().unwrap()
        );
        let start = DateTime::parse_from_rfc3339(&format!(
            "{}+01:00",
            e.get("event_starts_at").unwrap().as_str().unwrap()
        ))
        .unwrap();
        let end = DateTime::parse_from_rfc3339(&format!(
            "{}+01:00",
            e.get("event_ends_at").unwrap().as_str().unwrap()
        ))
        .unwrap();
        let hours = (end - start).num_minutes() as f64 / 60.0;
        let tarief = 18.0;
        let total = hours * tarief;
        subtotaal += total;
        template.push_str(&format!("{} | {} | {} | {}\n", desc, hours, tarief, total));
    }
    template.push_str(&format!("Subtotaal | | | {}\n", subtotaal));
    template.push_str(&format!("21% BTW | | | {}\n", 0.21 * subtotaal));
    template.push_str(&format!("Totaal | | | {}\n", subtotaal * 1.21));

    // Write invoice to a file
    let mut file = File::create("invoice.md").unwrap();
    file.write_all(template.as_bytes()).unwrap();
}
