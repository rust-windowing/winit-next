// MIT/Apache2 License

use async_process::Command;
use std::env;
use std::ffi::OsStr;

/// `git`
#[inline]
pub fn git() -> Command {
    command_with_env("GIT", "git")
}

/// `rustfmt`
#[inline]
pub fn rustfmt() -> Command {
    command_with_env("RUSTFMT", "rustfmt")
}

/// `cargo`
#[inline]
pub fn cargo() -> Command {
    command_with_env("CARGO", "cargo")
}

/// Get a command based on an environment variable.
#[inline]
fn command_with_env(env_name: &str, alterative: impl AsRef<OsStr>) -> Command {
    Command::new(
        env::var_os(env_name)
            .as_deref()
            .unwrap_or_else(|| alterative.as_ref()),
    )
}
