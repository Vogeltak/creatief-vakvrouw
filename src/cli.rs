use clap::Parser;

#[derive(Parser, Debug)]
#[clap(author, version, about)]
pub(crate) struct Cli {
    #[arg(short, long)]
    pub(crate) month: String,
    #[arg(short, long, default_value = "Noemi")]
    pub(crate) name: String,
}