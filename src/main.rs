use anyhow::Result;
use chrono::DateTime;
use clap::Parser;

use creatief_vakvrouw::anita;
use creatief_vakvrouw::cli;
use creatief_vakvrouw::event;
use creatief_vakvrouw::server;

#[tokio::main]
async fn main() -> Result<()> {
    let arg = cli::Cli::parse();

    match arg.command {
        cli::Commands::Anita { month, name } => get_anita(month, name).await,
        cli::Commands::Server => server::run().await,
    }
}

async fn get_anita(month: String, name: String) -> Result<()> {
    let (year, month) = month.split_once('-').unwrap();

    let rooster_noemi = anita::Anita::new(name);
    let events = rooster_noemi
        .get_events_from_month(month.to_owned(), year.to_owned())
        .await?;

    for e in &events {
        println!("{} works at {} from {}", e.person, e.date, e.start_to_end);
    }

    if events.is_empty() {
        return Err(anyhow::anyhow!("No relevant events"));
    }

    println!();
    invoice(&events);

    Ok(())
}

fn invoice(events: &[event::Event]) {
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
