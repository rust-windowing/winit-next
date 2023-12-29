// MIT/Apache2 License

//! Run the tests on an Android emulator, using xbuild and adb.
//!
//! Only works on Linux.

use super::{CurrentHost, Environment};

use async_process::{Child, ChildStderr, ChildStdin, ChildStdout, Command};
use color_eyre::eyre::Result;
use once_cell::sync::OnceCell;

use std::ffi::OsStr;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Runs an Android emulator in Docker and communicates with it.
pub struct AndroidEnvironment {
    /// The cell containing the Docker container running the Android system.
    android_runner: OnceCell<AndroidRunner>,

    /// Run commands on the current host.
    host: CurrentHost,
}

struct AndroidRunner {
    /// The docker process.
    docker: Child,

    /// The child's standard output.
    child_stdout: Mutex<ChildStdout>,

    /// The child's standard error.
    child_stderr: Mutex<ChildStderr>,
}

impl AndroidEnvironment {
    /// Create a new Android runner.
    #[inline]
    pub fn new(root: PathBuf) -> Self {
        Self {
            android_runner: OnceCell::new(),
            host: CurrentHost::new(root),
        }
    }

    #[inline]
    async fn setup_android_emulator(&self) -> Result<()> {
        Ok(())
    }
}

impl Environment for AndroidEnvironment {
    type Command = Child;

    fn cleanup(
        &self,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<()>> + Send + '_>>
    {
        Box::pin(async move {
            // TODO
            Ok(())
        })
    }

    fn run_command(&self, cmd: &OsStr, args: &[&OsStr]) -> Result<Self::Command> {
        // For `cargo test --tests` and `cargo test --doc`, we can't actually run these on Android.
        // Just skip them for now.
        if cmd.to_str().map_or(false, |s| s.ends_with("cargo"))
            && args.first().and_then(|arg| arg.to_str()) == Some("test")
        {
            tracing::warn!("cannot run `cargo test` on Android, ignoring");
            return Ok(Command::new("true").spawn()?);
        }

        todo!()
    }
}
