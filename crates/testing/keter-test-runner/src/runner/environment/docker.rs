// MIT/Apache2 License

//! Run tests inside of a Docker container with a configured environment.
//!
//! Useful for Linux/Windows tests on the same architecture.

use async_process::Child;
use color_eyre::eyre::{bail, eyre, Result};

use futures_lite::io::BufReader;
use futures_lite::prelude::*;

use std::ffi::OsStr;
use std::path::PathBuf;

use crate::runner::command::{docker, run};
use crate::runner::environment::{CurrentHost, Environment, RunCommand};
use crate::runner::util::spawn;

/// Run commands in a Docker container.
pub(crate) struct DockerEnvironment {
    /// Host to run commands on.
    host: CurrentHost,

    /// The ID of the Docker container.
    docker_id: String,
}

impl DockerEnvironment {
    /// Start up the Docker image.
    pub(crate) async fn start(
        root: PathBuf,
        target_triple: &str,
        options: Option<&str>,
    ) -> Result<Self> {
        let host = CurrentHost::new(root.clone());

        let root = root
            .to_str()
            .ok_or_else(|| eyre!("cannot have root be a non-UTF8 path for Docker environment"))?;

        // Start the docker container.
        let mut child = docker()?
            .arg("--detach")
            .args(["--volume", &format!("{root}:{root}")])
            .args(["--workdir", root])
            .arg(get_target_container(target_triple, options)?)
            .spawn(&host)?;

        // Read stdout to get the container ID.
        let container_id = {
            let mut stdout = child.stdout.take().unwrap();
            let mut buf = String::new();

            // Read to end and then wait for finish.
            let docker_runner = spawn(async move { child.exit().await });
            stdout.read_to_string(&mut buf).await?;
            docker_runner.await?;

            // Buffer should contain the container ID.
            buf
        };

        Ok(Self {
            host,
            docker_id: container_id,
        })
    }
}

impl Environment for DockerEnvironment {
    type Command = Child;

    #[inline]
    fn cleanup(&self) -> std::pin::Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        Box::pin(async move {
            // Run a process to stop the docker container.
            run(
                "docker stop",
                docker()?
                    .arg("stop")
                    .arg(&self.docker_id)
                    .spawn(&self.host)?,
                None,
            )
            .await?;

            // Clean up the Docker container.
            run(
                "docker rm",
                docker()?.arg("rm").arg(&self.docker_id).spawn(&self.host)?,
                None,
            )
            .await?;

            Ok(())
        })
    }

    #[inline]
    fn run_command(&self, cmd: &OsStr, args: &[&OsStr]) -> Result<Self::Command> {
        todo!()
    }
}

fn get_target_container(target_triple: &str, options: Option<&str>) -> Result<String> {
    bail!("no known container for target triple {target_triple} and options {options:?}")
}
