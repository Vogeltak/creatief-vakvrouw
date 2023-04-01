use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[clap(author, version, about)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Anita {
        #[arg(short, long)]
        month: String,
        #[arg(short, long, default_value = "Noemi")]
        name: String,
    },
    Server,
}