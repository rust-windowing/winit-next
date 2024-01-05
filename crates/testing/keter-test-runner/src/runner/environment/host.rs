// MIT/Apache2 License

//! Run the commands on the current host system.

use super::{Environment, RunCommand};

use async_process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, Stdio};
use color_eyre::eyre::{bail, eyre, Result};
use futures_lite::prelude::*;

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::pin::Pin;

/// Run commands directly on the current host.
pub(crate) struct CurrentHost {
    root: PathBuf,
}

impl CurrentHost {
    pub(crate) fn new(path: PathBuf) -> Self {
        Self { root: path }
    }
}

impl Environment for CurrentHost {
    type Command = Child;

    fn run_command(
        &self,
        cmd: &OsStr,
        args: &[&OsStr],
        pwd: Option<&OsStr>,
    ) -> Result<Self::Command> {
        tracing::info!("running command {cmd:?} with args {args:?}",);

        let mut command = Command::new(cmd);
        command.args(args);
        if let Some(pwd) = pwd {
            command.current_dir(pwd);
        }
        let child = command
            .current_dir(&self.root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::piped())
            .spawn()?;

        Ok(child)
    }

    fn cleanup(&self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        Box::pin(std::future::ready(Ok(())))
    }
}

impl RunCommand for Child {
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
            // Wait for the child to complete.
            let status = self.status().await?;
            if !status.success() {
                bail!("child exited with error code {status:?}");
            }

            Ok(())
        })
    }
}
