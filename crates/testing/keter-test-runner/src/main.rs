// MIT/Apache2 License

mod runner;

use color_eyre::Result;
use owo_colors::OwoColorize;

fn main() {
    // Setup error hooks.
    tracing_subscriber::fmt::try_init().ok();
    color_eyre::install().ok();

    // Run the main function.
    if let Err(e) = async_io::block_on(entry()) {
        println!("{}{}", "encountered a fatal error: ".red().bold(), e);
    }
}

async fn entry() -> Result<()> {
    runner::Test::Style.run().await?;
    Ok(())
}
