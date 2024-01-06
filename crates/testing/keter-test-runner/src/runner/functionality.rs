// MIT/Apache2 License

//! Functionality tests.
//!
//! This just runs "cargo test" in the proper host environment.

use crate::runner::command::{cargo_for_check, run};
use crate::runner::environment::choose_environment;
use crate::runner::Crate;

use color_eyre::eyre::{Result, WrapErr};

use std::path::Path;
use std::time::Duration;

const FUNCTEST_TIMEOUT: Duration = Duration::from_secs(5 * 60);

/// Run functionality tests.
pub async fn functionality(root: &Path, crates: Vec<Crate>) -> Result<()> {
    for crate_ in crates {
        for check in &crate_.checks {
            // Choose an environment for this check.
            let host = choose_environment(root, check)
                .await
                .context("while choosing environment")?;

            for mode in ["--tests", "--doc"] {
                // Run the cargo command.
                let mut command = cargo_for_check(&["test", mode], &crate_, check)?;
                run(
                    "cargo_functionality",
                    command.spawn(&*host)?,
                    Some(FUNCTEST_TIMEOUT),
                )
                .await
                .with_context(|| format!("while running cargo test {mode}"))?;
            }
        }
    }

    Ok(())
}
