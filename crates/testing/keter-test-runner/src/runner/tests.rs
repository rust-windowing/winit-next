// MIT/Apache2 License

//! Run the test suite found in `keter-test`.

use crate::runner::command::{cargo, cargo_for_check, run};
use crate::runner::environment::{choose_environment, CurrentHost, Environment, RunCommand};
use crate::runner::util::spawn;

use async_executor::Task;
use async_lock::OnceCell;
use color_eyre::eyre::{eyre, Context, Result};
use futures_lite::prelude::*;
use serde::Deserialize;
use std::path::{Path, PathBuf};

use futures_lite::io::BufReader;
use futures_lite::prelude::*;

use crate::runner::{Check, Crate};

pub(crate) async fn tests(root: &Path, crates: Vec<Crate>) -> Result<()> {
    for crate_ in crates {
        // Get the root directory for this crate.
        let crate_root = crate_manifest_root(root, &crate_.name).await?;

        // There should be a folder named "keter-tests" here. If there isn't, there are no tests.
        let keter_tests = crate_root.join("keter_tests");
        if async_fs::metadata(&keter_tests).await.is_err() {
            tracing::warn!("crate `{}` has no keter_tests", &crate_.name);
            continue;
        }

        // List the examples here in keter tests.
        let mut touched = false;
        let running_tests = async_fs::read_dir(keter_tests)
            .await?
            .inspect(|_| {
                touched = true;
            })
            .then({
                let crate_ = &crate_;
                move |example| async move {
                    let example = example?.path();
                    let name = example
                        .file_name()
                        .and_then(|name| name.to_str())
                        .ok_or_else(|| {
                            eyre!("encountered invalid keter_test example: {example:?}")
                        })?;

                    for check in &crate_.checks {
                        let env = choose_environment(root, check).await?;
                        let example_name = format!("{}_{}", crate_.name, name);

                        let mut cmd =
                            cargo_for_check(&["run", "--example", &example_name], crate_, check)?;
                        run(&format!("cargo_test_{name}"), cmd.spawn(&*env)?, None)
                            .await
                            .with_context(|| format!("while running keter test {name}"))?;
                    }

                    Ok(())
                }
            });

        futures_lite::pin!(running_tests);
        running_tests
            .try_for_each(|r: color_eyre::eyre::Result<()>| r)
            .await?;

        if !touched {
            tracing::warn!("no keter tests run for {}", &crate_.name);
        }
    }

    Ok(())
}

/// Get the root of a specific crate name.
async fn crate_manifest_root(root: &Path, name: &str) -> Result<&'static Path> {
    let metadata = cargo_metadata(root).await?;

    for pack in &metadata.packages {
        if pack.name == name {
            return pack
                .manifest_path
                .parent()
                .ok_or_else(|| eyre!("manifest path should never be root"));
        }
    }

    Err(eyre!("unable to find package {name}"))
}

/// Get the output of `cargo metadata`.
async fn cargo_metadata(root: &Path) -> Result<&'static CargoMetadata> {
    static CARGO_METADATA: OnceCell<CargoMetadata> = OnceCell::new();

    CARGO_METADATA
        .get_or_try_init(|| async {
            // Launch the child.
            let host = CurrentHost::new(root.to_path_buf());
            let mut child = cargo()?.arg("metadata").spawn(host)?;

            // Run the process.
            let mut stdout = child.stdout().unwrap();
            let stdout = spawn(async move {
                let mut buf = Vec::new();
                stdout.read_to_end(&mut buf).await?;
                std::io::Result::Ok(buf)
            });
            run("cargo metadata", child, None).await?;

            // Finish reading stdout.
            let package_metadata = stdout.await?;

            // Parse stdout.
            let meta = serde_json::from_slice(&package_metadata)?;
            Ok(meta)
        })
        .await
}

#[derive(Deserialize)]
struct CargoMetadata {
    packages: Vec<MetadataPackage>,
}

#[derive(Deserialize)]
struct MetadataPackage {
    name: String,
    manifest_path: PathBuf,
}
