// MIT/Apache2 License

mod runner;

use std::path::Path;

use color_eyre::{eyre::bail, Result};
use owo_colors::OwoColorize;

fn main() {
    // Setup error hooks.
    tracing_subscriber::fmt::try_init().ok();
    color_eyre::install().ok();

    // Get CLI matches.
    let matches = cli().get_matches();

    // Run the main function.
    if let Err(e) = async_io::block_on(entry(matches)) {
        println!("{}{}", "encountered a fatal error: ".red().bold(), e);
    }
}

async fn entry(matches: clap::ArgMatches) -> Result<()> {
    match matches.subcommand() {
        None => bail!("expected a subcommand"),
        Some(("style", matches)) => {
            let crates = read_config(matches.get_one::<String>("config").unwrap()).await?;
            runner::Test::Style.run(crates).await?;
        }
        Some((subcommand, _matches)) => bail!("unknown subcommand {subcommand}"),
    }

    Ok(())
}

async fn read_config(path: impl AsRef<Path>) -> Result<Vec<runner::Crate>> {
    let data = async_fs::read(path).await?;
    let crates = serde_json::from_slice(&data)?;
    Ok(crates)
}

fn cli() -> clap::Command {
    clap::Command::new("keter-test-runner").subcommand(
        clap::Command::new("style").arg(
            clap::Arg::new("config")
                .required(true)
                .short('c')
                .long("config"),
        ),
    )
}
