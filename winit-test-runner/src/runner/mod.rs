// MIT/Apache2 License

mod command;
mod environment;
mod functionality;
mod style;
mod tests;
mod util;

use color_eyre::eyre::eyre;
use serde::{Deserialize, Serialize};

use std::path::Path;

/// A crate to test.
#[derive(Serialize, Deserialize, Debug)]
pub struct Crate {
    /// Crate name to test.
    pub name: String,

    /// Checks to run.
    pub checks: Vec<Check>,
}

/// Check to run for a crate.
#[derive(Serialize, Deserialize, Debug)]
pub struct Check {
    /// The target triple to test.
    #[serde(rename = "target")]
    pub target_triple: String,

    /// Host environment to set up.
    pub host_env: Option<String>,

    /// Features to enable.
    pub features: Option<Vec<String>>,

    /// Turn off default features.
    #[serde(default)]
    pub no_default_features: bool,

    /// Whether this test should be ignored in the general CI case.
    #[serde(default)]
    pub niche: bool,
}

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
    pub async fn run(self, crates: Vec<Crate>) -> color_eyre::Result<()> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(1)
            .ok_or_else(|| eyre!("this cargo package is at an invalid path"))?;

        let result = match self {
            Self::Style => util::run(style::style(root, crates)).await,
            Self::Functionality => util::run(functionality::functionality(root, crates)).await,
            Self::Host => util::run(tests::tests(root, crates)).await,
        };

        environment::cleanup_hosts().await?;

        result
    }
}
