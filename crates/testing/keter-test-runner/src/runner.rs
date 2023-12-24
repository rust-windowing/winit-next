// MIT/Apache2 License

mod command;
mod style;

use color_eyre::eyre::eyre;
use std::path::Path;

/// Test type to run.
pub enum Test {
    /// Run style tests to make sure everything is properly formatted and linted.
    Style,

    /// Run functionality tests to make sure unit tests pass.
    Functionality,

    /// Run full tests on the current host machine
    Host,
}

impl Test {
    /// Run this test.
    pub async fn run(self) -> color_eyre::Result<()> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(3)
            .ok_or_else(|| eyre!("this cargo package is at an invalid path"))?;

        match self {
            Self::Style => style::style(root).await?,
            _ => todo!(),
        }

        Ok(())
    }
}
