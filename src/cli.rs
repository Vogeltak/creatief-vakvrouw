use clap::Parser;

#[derive(Parser, Debug)]
#[clap(author, version, about)]
pub struct Cli {
    #[arg(short, long)]
    pub month: String,
    #[arg(short, long, default_value = "Noemi")]
    pub name: String,
}