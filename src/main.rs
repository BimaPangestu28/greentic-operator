use clap::Parser;
use greentic_operator::cli;

fn main() -> anyhow::Result<()> {
    let cli = cli::Cli::parse();
    cli.run()
}
