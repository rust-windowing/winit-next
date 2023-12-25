// MIT/Apache2 License

//! Host environment to run tests in.
//!
//! This allows the test runner to issue commands to another system.

mod host;

use crate::runner::Check;

use color_eyre::Result;
use futures_lite::prelude::*;

use std::ffi::OsStr;
use std::path::Path;
use std::pin::Pin;

pub(crate) use host::CurrentHost;

/// Choose an environment to run inside of.
pub(crate) async fn choose_environment(root: &Path, check: &Check) -> Result<DynEnvironment> {
    // TODO: Other environments.
    let _ = check;
    tracing::info!("choosing to run on host operating system");
    Ok(DynEnvironment::from_environment(CurrentHost::new(
        root.to_path_buf(),
    )))
}

/// Host environment to run commands in.
pub(crate) trait Environment {
    /// The command to run.
    type Command: RunCommand + Send + 'static;

    /// Run a command.
    fn run_command(&mut self, cmd: &OsStr, args: &[&OsStr]) -> Result<Self::Command>;
}

impl<E: Environment + ?Sized> Environment for &mut E {
    type Command = E::Command;

    #[inline]
    fn run_command(&mut self, cmd: &OsStr, args: &[&OsStr]) -> Result<Self::Command> {
        (**self).run_command(cmd, args)
    }
}

/// A command to run.
pub(crate) trait RunCommand {
    /// Get a writer for the standard input.
    fn stdin(&mut self) -> Option<Pin<Box<dyn AsyncWrite + Send + 'static>>>;
    /// Get a reader for the standard output.
    fn stdout(&mut self) -> Option<Pin<Box<dyn AsyncRead + Send + 'static>>>;
    /// Get a reader for the standard output.
    fn stderr(&mut self) -> Option<Pin<Box<dyn AsyncRead + Send + 'static>>>;

    /// Wait for this child to exit.
    fn exit(&mut self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>>;
}

impl<RC: RunCommand + ?Sized> RunCommand for Box<RC> {
    fn stdin(&mut self) -> Option<Pin<Box<dyn AsyncWrite + Send + 'static>>> {
        (**self).stdin()
    }
    fn stdout(&mut self) -> Option<Pin<Box<dyn AsyncRead + Send + 'static>>> {
        (**self).stdout()
    }
    fn stderr(&mut self) -> Option<Pin<Box<dyn AsyncRead + Send + 'static>>> {
        (**self).stderr()
    }
    fn exit(&mut self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        (**self).exit()
    }
}

/// A dynamically allocated environment.
pub struct DynEnvironment {
    inner: Box<dyn Environment<Command = Box<dyn RunCommand + Send + 'static>> + Send + 'static>,
}

impl DynEnvironment {
    /// Create a new `DynEnvironment` from an existing `Environment`.
    pub fn from_environment(env: impl Environment + Send + 'static) -> Self {
        struct BoxedRunCommandEnvironment<E>(E);

        impl<E: Environment> Environment for BoxedRunCommandEnvironment<E> {
            type Command = Box<dyn RunCommand + Send + 'static>;

            #[inline]
            fn run_command(&mut self, cmd: &OsStr, args: &[&OsStr]) -> Result<Self::Command> {
                let cmd = self.0.run_command(cmd, args)?;
                Ok(Box::new(cmd))
            }
        }

        Self {
            inner: Box::new(BoxedRunCommandEnvironment(env)),
        }
    }
}

impl Environment for DynEnvironment {
    type Command = Box<dyn RunCommand + Send + 'static>;

    #[inline]
    fn run_command(&mut self, cmd: &OsStr, args: &[&OsStr]) -> Result<Self::Command> {
        self.inner.run_command(cmd, args)
    }
}
