// MIT/Apache2 License

//! Functionality tests.
//!
//! This just runs "cargo test" in the proper host environment.

use crate::runner::command::{cargo_for_check, run};
use crate::runner::environment::{choose_environment, Environment};
use crate::runner::Crate;

use color_eyre::eyre::Result;

use std::path::Path;
use std::time::Duration;

const FUNCTEST_TIMEOUT: Duration = Duration::from_secs(5 * 60);

/// Run functionality tests.
pub async fn functionality(root: &Path, crates: Vec<Crate>) -> Result<()> {
    for crate_ in crates {
        for check in &crate_.checks {
            // Choose an environment for this check.
            let mut host = choose_environment(root, check).await?;

            for mode in ["--tests", "--docs"] {
                // Run the cargo command.
                let mut command = cargo_for_check(&["test", mode], &crate_, check)?;
                run(
                    "cargo_functionality",
                    command.spawn(&mut host)?,
                    Some(FUNCTEST_TIMEOUT),
                )
                .await?;
            }
        }
    }

    Ok(())
}
