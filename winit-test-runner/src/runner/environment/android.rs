// MIT/Apache2 License

//! Run the tests on an Android emulator, using xbuild and adb.
//!
//! Only works on Linux.

use super::{CurrentHost, Environment, RunCommand};

use crate::runner::command::{adb, docker, run, xbuild};
use crate::runner::util::spawn;

use async_executor::Task;
use async_lock::OnceCell;
use color_eyre::eyre::{bail, Context, Result};
use regex::Regex;

use futures_lite::io::BufReader;
use futures_lite::prelude::*;

use std::env;
use std::ffi::OsStr;
use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

const ANDROID_DOCKER_IMAGE: &str =
    "us-docker.pkg.dev/android-emulator-268719/images/30-google-x64:30.1.2";

const SHORT_COMMAND_TIMEOUT: Duration = Duration::from_secs(10);
const WAIT_TIMEOUT: Duration = Duration::from_secs(5 * 60);

/// Runs an Android emulator in Docker and communicates with it.
#[derive(Clone)]
pub struct AndroidEnvironment {
    /// The cell containing the Docker container running the Android system.
    android_runner: Arc<OnceCell<AndroidRunner>>,

    /// Run commands on the current host.
    host: Arc<CurrentHost>,
}

struct AndroidRunner {
    /// The docker ID.
    docker_id: String,
}

impl AndroidEnvironment {
    /// Create a new Android runner.
    #[inline]
    pub fn new(root: PathBuf) -> Self {
        Self {
            android_runner: Arc::new(OnceCell::new()),
            host: Arc::new(CurrentHost::new(root)),
        }
    }

    #[inline]
    async fn setup_android_emulator(&self) -> Result<AndroidRunner> {
        // Read the adbkey file in home.
        let adbkey = {
            let adbkey_path = PathBuf::from(env::var_os("HOME").unwrap()).join(".android/adbkey");
            let mut adbkey_buffer = String::new();
            async_fs::File::open(adbkey_path)
                .await?
                .read_to_string(&mut adbkey_buffer)
                .await?;
            adbkey_buffer
        };

        // Start the docker image.
        let mut docker_run = docker()?
            .arg("run")
            .arg("--detach")
            .arg("-e")
            .arg(format!("ADBKEY={adbkey}"))
            .args(["--device", "/dev/kvm"])
            .args(["--publish", "8554:8554/tcp"])
            .args(["--publish", "15555:5555/tcp"])
            .arg(ANDROID_DOCKER_IMAGE)
            .spawn(&*self.host)?;

        // Read stdout to get the container ID
        let container_id = {
            let mut stdout = docker_run.stdout.take().unwrap();
            let stdout_runner = spawn(async move {
                let mut buf = String::new();
                stdout.read_to_string(&mut buf).await?;
                std::io::Result::Ok(buf)
            });

            run(
                "android docker container spawn",
                docker_run,
                Some(SHORT_COMMAND_TIMEOUT),
            )
            .await?;

            let mut container_id = stdout_runner.await?;
            if container_id.ends_with('\n') {
                container_id.pop();
            }
            container_id
        };

        // Wait for Docker to start running.
        async_io::Timer::after(Duration::from_millis(100)).await;

        // Initialize ADB connecting to host port 15555 (adb inside the container).
        let adb_connect = || async {
            run(
                "adb connect localhost:15555",
                adb()?
                    .arg("connect")
                    .arg("localhost:15555")
                    .spawn(&*self.host)?,
                Some(SHORT_COMMAND_TIMEOUT),
            )
            .await
        };
        for i in 0..5 {
            match adb_connect().await {
                Ok(()) => break,
                Err(err) => {
                    if i == 5 {
                        return Err(err);
                    } else {
                        tracing::error!("adb connect failed, retrying in two seconds...");
                        async_io::Timer::after(Duration::from_secs(2)).await;
                        continue;
                    }
                }
            }
        }

        // Wait for the device to come online.
        run(
            "adb wait-for-device",
            adb()?.arg("wait-for-device").spawn(&*self.host)?,
            Some(WAIT_TIMEOUT),
        )
        .await?;

        // Wait for the boot to complete.
        {
            let mut retry = 0;
            loop {
                let mut boot_check = adb()?
                    .arg("shell")
                    .arg("getprop")
                    .arg("sys.boot_completed")
                    .spawn(&*self.host)?;

                let mut stdout = boot_check.stdout.take().unwrap();
                let runner = spawn(async move {
                    run(
                        "adb shell getprop sys.boot_completed",
                        boot_check,
                        Some(SHORT_COMMAND_TIMEOUT),
                    )
                    .await
                });

                let mut result = String::new();
                stdout.read_to_string(&mut result).await?;
                runner.await?;

                // If the first char is `1`, we are done.
                if result.starts_with('1') {
                    break;
                }

                // Otherwise, try again.
                retry += 1;
                if retry >= 240 {
                    bail!("failed to get boot status after 240 tries");
                }

                async_io::Timer::after(Duration::from_secs(2)).await;
            }
        };

        Ok(AndroidRunner {
            docker_id: container_id,
        })
    }
}

impl Environment for AndroidEnvironment {
    type Command = AndroidCommand;

    fn cleanup(&self) -> std::pin::Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        Box::pin(async move {
            if let Some(runner) = self.android_runner.get() {
                // Run a process to stop the docker container.
                run(
                    "docker stop",
                    docker()?
                        .arg("stop")
                        .arg(&runner.docker_id)
                        .spawn(&*self.host)
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
                        .arg(&runner.docker_id)
                        .spawn(&*self.host)
                        .context("while spawning docker rm")?,
                    None,
                )
                .await
                .context("while spawning docker rm")?;
            }

            Ok(())
        })
    }

    fn run_command(
        &self,
        cmd: &OsStr,
        args: &[&OsStr],
        pwd: Option<&OsStr>,
    ) -> Result<Self::Command> {
        assert!(pwd.is_none());
        let is_cargo = cmd.to_str().map_or(false, |s| s.ends_with("cargo"));

        // For `cargo test --tests` and `cargo test --doc`, we can't actually run these on Android.
        // Just skip them for now.
        if is_cargo && args.first().and_then(|arg| arg.to_str()) == Some("test") {
            tracing::warn!("cannot run `cargo test` on Android, ignoring");
            return Ok(AndroidCommand::NoOp);
        }

        // For `cargo run --example`, we need to run the example itself.
        if is_cargo && args.first().and_then(|arg| arg.to_str()) == Some("run") {
            let this = self.clone();
            let task = spawn(async move {
                // Start up the Android emulator.
                SendSyncWrapper {
                    f: this
                        .android_runner
                        .get_or_try_init(|| async { this.setup_android_emulator().await }),
                }
                .await?;

                // Spawn a child.
                let mut xbuild = xbuild()?
                    .arg("run")
                    .args(["--device", "adb:localhost:15555"])
                    .args(["--arch", "arm64"])
                    .args([
                        "--manifest-path",
                        "crates/foundation/winit-reactor/winit_tests/general_tests/Cargo.toml",
                    ])
                    .spawn(&*this.host)?;

                let (line_sender, line_receiver) = async_channel::bounded(1);

                // Take out stdout and stderr and analyze them.
                let ls = line_sender.clone();
                let mut stdout = BufReader::new(xbuild.stdout.take().unwrap());
                let stdout = spawn(async move {
                    loop {
                        let mut buf = String::new();
                        if stdout.read_line(&mut buf).await.is_err() {
                            break;
                        }
                        if buf.is_empty() {
                            break;
                        }

                        if buf.ends_with('\n') {
                            buf.pop();
                        }
                        tracing::trace!("xbuild stdout: {buf}");

                        ls.send(buf).await.ok();
                    }
                });

                let mut stderr = BufReader::new(xbuild.stderr.take().unwrap());
                let stderr = spawn(async move {
                    loop {
                        let mut buf = String::new();
                        if stderr.read_line(&mut buf).await.is_err() {
                            break;
                        }
                        if buf.is_empty() {
                            break;
                        }

                        if buf.ends_with('\n') {
                            buf.pop();
                        }
                        tracing::trace!("xbuild stderr: {buf}");

                        line_sender.send(buf).await.ok();
                    }
                });

                let runner = spawn(async move { run("xbuild", xbuild, None).await });
                let mut reporter = winit_test::reporter::ConsoleReporter::new();
                let dump_finder = Regex::new(r"winit_TEST_DUMP\((.*)\)winit_TEST_DUMP")?;

                let regex_finder = async {
                    while let Ok(line) = line_receiver.recv().await {
                        if let Some(mat) = dump_finder.captures(&line) {
                            if let Some(data) = mat.get(1) {
                                let mut stop_running = false;
                                let event: winit_test::TestEvent =
                                    serde_json::from_str(data.as_str())?;

                                // Stop running if event is the end event.
                                if let winit_test::TestEvent::End { .. } = &event {
                                    stop_running = true;
                                }

                                winit_test::reporter::Reporter::report(&mut reporter, event).await;

                                if stop_running {
                                    break;
                                }
                            }
                        }
                    }

                    Ok(())
                };

                let result = regex_finder.await;

                // Cancel tasks.
                if let Some(Err(e)) = runner.cancel().await {
                    return Err(e);
                }
                stdout.cancel().await;
                stderr.cancel().await;

                let code = winit_test::reporter::Reporter::finish(&mut reporter);
                if code != 0 {
                    bail!("received an error from the android runner")
                } else {
                    result
                }
            });

            return Ok(AndroidCommand::XbuildRun(task));
        }

        bail!("unable to run Android command: {cmd:?} {args:?}")
    }
}

pub(crate) enum AndroidCommand {
    NoOp,
    XbuildRun(Task<color_eyre::eyre::Result<()>>),
}

impl RunCommand for AndroidCommand {
    #[inline]
    fn exit(&mut self) -> std::pin::Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        Box::pin(async move {
            match self {
                Self::NoOp => Ok(()),
                Self::XbuildRun(ch) => ch.await,
            }
        })
    }

    #[inline]
    fn stdin(&mut self) -> Option<std::pin::Pin<Box<dyn AsyncWrite + Send + 'static>>> {
        None
    }

    #[inline]
    fn stderr(&mut self) -> Option<std::pin::Pin<Box<dyn AsyncRead + Send + 'static>>> {
        // Always taken out.
        None
    }

    #[inline]
    fn stdout(&mut self) -> Option<std::pin::Pin<Box<dyn AsyncRead + Send + 'static>>> {
        // Always taken out.
        None
    }
}

pin_project_lite::pin_project! {
    // https://github.com/smol-rs/event-listener-strategy/issues/13
    struct SendSyncWrapper<F> {
        #[pin]
        f: F
    }
}

unsafe impl<F> Send for SendSyncWrapper<F> {}
unsafe impl<F> Sync for SendSyncWrapper<F> {}

impl<F: Future> Future for SendSyncWrapper<F> {
    type Output = F::Output;

    #[inline]
    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        self.project().f.poll(cx)
    }
}
