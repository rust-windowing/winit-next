// MIT/Apache2 License

//! Run the commands on the current host system.

use super::{Environment, RunCommand};

use async_process::{Child, ChildStderr, ChildStdin, ChildStdout, Command};
use color_eyre::eyre::{bail, eyre, Result};
use futures_lite::prelude::*;

use std::ffi::OsStr;
use std::path::Path;
use std::pin::Pin;

/// Run commands directly on the current host.
pub(crate) struct CurrentHost;

impl CurrentHost {
    pub(crate) fn new() -> Self {
        Self
    }
}

/// Wrapper around `Command` to fit the `RunCommand` API.
pub(crate) struct CurrentHostCommand {
    /// Current child process.
    child: Option<Child>,

    /// Standard input.
    stdin: Option<ChildStdin>,

    /// Standard output.
    stdout: Option<ChildStdout>,

    /// Standard error.
    stderr: Option<ChildStderr>,
}

impl Environment for CurrentHost {
    type Command = CurrentHostCommand;

    fn run_command(&mut self, cmd: &OsStr, args: &[&OsStr], pwd: &Path) -> Result<Self::Command> {
        let mut child = Command::new(cmd).args(args).current_dir(pwd)
            .spawn()?;
        
        Ok(CurrentHostCommand {
            stdin: child.stdin.take(),
            stdout: child.stdout.take(),
            stderr: child.stderr.take(),
            child: Some(child)
        })
    }
}

impl RunCommand for CurrentHostCommand {
    fn stdin(&mut self) -> Option<Pin<Box<dyn AsyncWrite + Send + 'static>>> {
        let stdin = self.stdin.take()?;
        Some(Box::pin(stdin))
    }

    fn stdout(&mut self) -> Option<Pin<Box<dyn AsyncRead + Send + 'static>>> {
        let stdout = self.stdout.take()?;
        Some(Box::pin(stdout))
    }

    fn stderr(&mut self) -> Option<Pin<Box<dyn AsyncRead + Send + 'static>>> {
        let stderr = self.stderr.take()?;
        Some(Box::pin(stderr))
    }

    fn exit(&mut self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        Box::pin(async move {
            let mut child = self
                .child
                .take()
                .ok_or_else(|| eyre!("cannot call exit() twice"))?;

            // Wait for the child to complete.
            let status = child.status().await?;
            if !status.success() {
                bail!("child exited with error code {status:?}");
            }

            Ok(())
        })
    }
}
