// MIT/Apache2 License

use super::environment::{Environment, RunCommand};
use super::util::spawn;
use super::{Check, Crate};

use color_eyre::eyre::{eyre, Result};
use tracing::Instrument;

use futures_lite::io::BufReader;
use futures_lite::prelude::*;

use std::env;
use std::ffi::{OsStr, OsString};
use std::time::Duration;

/// A command to run.
pub(crate) struct Command {
    /// The command to run.
    command: OsString,

    /// Arguments for the command.
    args: Vec<OsString>,
}

impl Command {
    /// Create a new command.
    pub(crate) fn new(command: impl AsRef<OsStr>) -> Self {
        Self {
            command: command.as_ref().to_os_string(),
            args: vec![],
        }
    }

    /// Add an argument to this command.
    #[inline]
    pub(crate) fn arg(&mut self, arg: impl AsRef<OsStr>) -> &mut Self {
        self.args.push(arg.as_ref().to_os_string());
        self
    }

    /// Add several arguments to this command.
    #[inline]
    pub(crate) fn args<S: AsRef<OsStr>>(&mut self, args: impl IntoIterator<Item = S>) -> &mut Self {
        let mut args = args.into_iter();
        let (lo, _) = args.size_hint();

        self.args.reserve(lo);
        for arg in args {
            self.args.push(arg.as_ref().to_os_string());
        }

        self
    }

    /// Run this command on a host environment.
    pub(crate) fn spawn<E: Environment>(&mut self, mut host: E) -> Result<E::Command> {
        let args = self.args.iter().map(|arg| &**arg).collect::<Vec<_>>();
        host.run_command(&self.command, args.as_slice())
    }
}

/// Run a command to completion.
#[inline]
pub async fn run(
    name: &str,
    mut child: impl RunCommand + Send + 'static,
    timeout: Option<Duration>,
) -> Result<()> {
    drop(child.stdin());

    // Spawn a task to emit stdout to tracing.
    let run_stdout = spawn({
        let stdout = child.stdout();
        let span = tracing::trace_span!("stdout", name);
        async move {
            if let Some(stdout) = stdout {
                let mut buffer = String::new();
                let mut stdout = BufReader::new(stdout);

                while stdout.read_line(&mut buffer).await.is_ok() {
                    if buffer.is_empty() {
                        break;
                    }
                    buffer.pop();
                    tracing::trace!("+ {buffer}");
                    buffer.clear();
                }

                // Write out any remaining data.
                if !buffer.is_empty() {
                    tracing::trace!("+ {buffer}");
                }
            }
        }
        .instrument(span)
    });

    // Spawn a task to emit stderr to info.
    let run_stderr = spawn({
        let stderr = child.stderr();
        let span = tracing::info_span!("stderr", name);
        async move {
            if let Some(stderr) = stderr {
                let mut buffer = String::new();
                let mut stderr = BufReader::new(stderr);

                while stderr.read_line(&mut buffer).await.is_ok() {
                    if buffer.is_empty() {
                        break;
                    }
                    buffer.pop();
                    tracing::info!("+ {buffer}");
                    buffer.clear();
                }

                // Write out any remaining data.
                if !buffer.is_empty() {
                    tracing::info!("+ {buffer}");
                }
            }
        }
        .instrument(span)
    });

    // Spawn a task to poll the process.
    let status = spawn({
        let name = name.to_string();
        let span = tracing::info_span!("status", name);
        async move { child.exit().await }.instrument(span)
    });

    // Use a future to time out.
    let timeout = async move {
        timeout
            .map_or_else(async_io::Timer::never, async_io::Timer::after)
            .await;
        Err(eyre!("child {name} timed out"))
    };

    let result = status.or(timeout).await;

    // Cancel the other two tasks.
    run_stdout.cancel().await;
    run_stderr.cancel().await;

    result
}

/// `rustfmt`
#[inline]
pub fn rustfmt() -> Result<Command> {
    command_with_env("RUSTFMT", "rustfmt")
}

/// `rustc`
#[inline]
pub fn rustc() -> Result<Command> {
    command_with_env("RUSTC", "rustc")
}

/// `cargo`
#[inline]
pub fn cargo() -> Result<Command> {
    command_with_env("CARGO", "cargo")
}

/// `cargo` for a specific `Crate` and `Check`.
#[inline]
pub fn cargo_for_check(subcommands: &[&str], crate_: &Crate, check: &Check) -> Result<Command> {
    let mut cargo = cargo()?;
    cargo.args(subcommands);
    cargo.args(["--package", &crate_.name]);
    cargo.args(["--target", &check.target_triple]);

    if check.no_default_features {
        cargo.arg("--no-default-features");
    }
    if let Some(features) = check.features.as_ref() {
        let features = features.join(",");
        cargo.args(["--features", &features]);
    }

    Ok(cargo)
}

/// `cargo` for a set of `Crate`s.
#[inline]
pub fn cargo_for_crate<'a>(
    subcommands: &'static [&'static str],
    crate_: &'a Crate,
) -> impl Iterator<Item = Result<Command>> + 'a {
    crate_
        .checks
        .iter()
        .map(|check| cargo_for_check(subcommands, crate_, check))
}

/// Get a command based on an environment variable.
#[inline]
fn command_with_env(env_name: &str, alterative: impl AsRef<OsStr>) -> Result<Command> {
    // Get the command name.
    let name = env::var_os(env_name).unwrap_or_else(|| alterative.as_ref().to_os_string());

    // TODO: Tell if we have it.

    Ok(Command::new(name))
}
