use clap::Parser;
use opi::Cli;

fn main() -> anyhow::Result<()> {
    Cli::parse().run()
}
