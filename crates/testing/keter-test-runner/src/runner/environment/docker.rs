// MIT/Apache2 License

//! Run tests inside of a Docker container with a configured environment.
//!
//! Useful for Linux/Windows tests on the same architecture.

use async_executor::Task;
use async_process::Child;
use color_eyre::eyre::{bail, eyre, Result, WrapErr};

use futures_lite::io::BufReader;
use futures_lite::prelude::*;

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use crate::runner::command::{docker, run};
use crate::runner::environment::{CurrentHost, Environment, RunCommand};
use crate::runner::util::spawn;

const TEST_LISTENER_PATH: &str = "/tmp/keter_test_listener.sock";

/// Run commands in a Docker container.
pub(crate) struct DockerEnvironment {
    /// Host to run commands on.
    host: CurrentHost,

    /// The ID of the Docker container.
    docker_id: String,

    /// The running listener task.
    listener_task: Mutex<Option<Task<()>>>,
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

        // Start a Unix command line listener.
        let (ready_send, ready_recv) = async_channel::bounded(1);
        let listener_task = spawn(async move {
            #[cfg(unix)]
            {
                async_fs::remove_file(TEST_LISTENER_PATH).await.ok();
                if let Err(e) = keter_test::run_unix_listener(
                    TEST_LISTENER_PATH.as_ref(),
                    keter_test::reporter::ConsoleReporter::new(),
                    ready_send,
                )
                .await
                {
                    tracing::error!("unable to run Unix listener: {e}");
                }
            }

            #[cfg(not(unix))]
            todo!("how to run docker sockets outside of Unix?")
        });
        ready_recv.recv().await.ok();

        // Start the docker container.
        let mut child = docker()?
            .arg("run")
            .arg("--detach")
            .args(["--volume", &format!("{root}:{root}")])
            .args(["--volume", &format!("{0}:{0}", TEST_LISTENER_PATH)])
            .args([
                "--env",
                &format!("KETER_TEST_UDS_SOCKET={}", TEST_LISTENER_PATH),
            ])
            .args(["--workdir", root])
            .arg(get_target_container(target_triple, options)?)
            .args(["sh", "-c", "tail -f /dev/null"])
            .spawn(&host)
            .context("while spawning initial docker")?;

        // Read stdout to get the container ID.
        let container_id = {
            let mut stdout = child.stdout.take().unwrap();
            let stderr = child.stderr.take().unwrap();
            let mut buf = String::new();

            // Read to end and then wait for finish.
            let docker_runner = spawn(async move { child.exit().await });
            let stderr_runner = spawn(async move {
                let mut line = String::new();
                let mut stderr = BufReader::new(stderr);
                while stderr.read_line(&mut line).await.is_ok() {
                    line.pop();
                    if line.is_empty() {
                        break;
                    }
                    tracing::trace!("docker stderr: {line}");
                }
            });
            stdout
                .read_to_string(&mut buf)
                .await
                .context("while reading from Docker daemon")?;
            docker_runner
                .await
                .context("while waiting for docker runner exit")?;
            stderr_runner.cancel().await;

            if buf.ends_with('\n') {
                buf.pop();
            }
            tracing::info!("running container: {buf}");

            // Buffer should contain the container ID.
            buf
        };

        // Wait for a second for the Docker container to start running.
        async_io::Timer::after(Duration::from_millis(100)).await;

        Ok(Self {
            host,
            docker_id: container_id,
            listener_task: Mutex::new(Some(listener_task)),
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
                    .spawn(&self.host)
                    .context("while spawning docker stop")?,
                None,
            )
            .await
            .context("while running docker stop")?;

            // Clean up the Docker container.
            run(
                "docker rm",
                docker()?
                    .arg("rm")
                    .arg(&self.docker_id)
                    .spawn(&self.host)
                    .context("while spawning docker rm")?,
                None,
            )
            .await
            .context("while spawning docker rm")?;

            // Stop the listener.
            let lt = self.listener_task.lock().unwrap().take();
            if let Some(lt) = lt {
                lt.cancel().await;
            }

            Ok(())
        })
    }

    #[inline]
    fn run_command(
        &self,
        cmd: &OsStr,
        args: &[&OsStr],
        pwd: Option<&OsStr>,
    ) -> Result<Self::Command> {
        assert!(pwd.is_none());
        let mut sh_command = Path::new(cmd)
            .file_name()
            .ok_or_else(|| eyre!("no file name for command"))?
            .to_str()
            .ok_or_else(|| eyre!("cmd was not valid utf-8"))?
            .to_string();
        for arg in args {
            let arg = arg
                .to_str()
                .ok_or_else(|| eyre!("arg was not valid utf-8"))?;

            sh_command.push(' ');
            sh_command.push_str(arg);
        }

        tracing::debug!("docker exec with command: {sh_command}");

        let child = docker()?
            .arg("exec")
            .arg(&self.docker_id)
            .arg("sh")
            .arg("-c")
            .arg(sh_command)
            .spawn(&self.host)
            .context("while spawning docker exec")?;

        Ok(child)
    }
}

fn get_target_container(target_triple: &str, options: Option<&str>) -> Result<String> {
    let tag = if target_triple.contains("linux") {
        if target_triple.ends_with("gnu") {
            // TODO: Fedora, etc etc
            "ubuntu"
        } else if target_triple.ends_with("musl") {
            "alpine"
        } else {
            bail!("unrecognized linux version {target_triple}")
        }
    } else {
        bail!("no tag for target triple {target_triple}")
    };

    // TODO: Modified images for host options.
    if options.is_some() {
        bail!("cannot handle options yet");
    }

    Ok(format!("ghcr.io/notgull/keter:{tag}"))
}
