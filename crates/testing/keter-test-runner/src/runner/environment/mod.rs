// MIT/Apache2 License

//! Host environment to run tests in.
//!
//! This allows the test runner to issue commands to another system.

mod android;
mod choose;
mod host;

use crate::runner::Check;

use color_eyre::Result;
use futures_lite::prelude::*;
use once_cell::sync::OnceCell;

use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::Path;
use std::pin::Pin;

pub(crate) use choose::{choose as choose_environment, cleanup as cleanup_hosts};
pub(crate) use host::CurrentHost;

/// Host environment to run commands in.
pub(crate) trait Environment {
    /// The command to run.
    type Command: RunCommand + Send + 'static;

    /// Run a command.
    fn run_command(&self, cmd: &OsStr, args: &[&OsStr]) -> Result<Self::Command>;

    /// Clean up the current environment.
    fn cleanup(&self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>>;
}

impl<E: Environment + ?Sized> Environment for &E {
    type Command = E::Command;

    #[inline]
    fn run_command(&self, cmd: &OsStr, args: &[&OsStr]) -> Result<Self::Command> {
        (**self).run_command(cmd, args)
    }

    #[inline]
    fn cleanup(&self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        (**self).cleanup()
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
    inner: Box<
        dyn Environment<Command = Box<dyn RunCommand + Send + 'static>> + Send + Sync + 'static,
    >,
}

impl DynEnvironment {
    /// Create a new `DynEnvironment` from an existing `Environment`.
    pub fn from_environment(env: impl Environment + Send + Sync + 'static) -> Self {
        struct BoxedRunCommandEnvironment<E>(E);

        impl<E: Environment> Environment for BoxedRunCommandEnvironment<E> {
            type Command = Box<dyn RunCommand + Send + 'static>;

            #[inline]
            fn run_command(&self, cmd: &OsStr, args: &[&OsStr]) -> Result<Self::Command> {
                let cmd = self.0.run_command(cmd, args)?;
                Ok(Box::new(cmd))
            }

            #[inline]
            fn cleanup(&self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
                self.0.cleanup()
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
    fn run_command(&self, cmd: &OsStr, args: &[&OsStr]) -> Result<Self::Command> {
        self.inner.run_command(cmd, args)
    }

    #[inline]
    fn cleanup(&self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        self.inner.cleanup()
    }
}
