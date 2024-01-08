// MIT/Apache2 License

use crate::runner::command::rustc;
use crate::runner::environment::{CurrentHost, RunCommand};
use async_executor::{Executor, Task};
use color_eyre::eyre::{eyre, Result};
use once_cell::sync::OnceCell;

use futures_lite::io::BufReader;
use futures_lite::prelude::*;

use std::future::{pending, Future};
use std::path::Path;
use std::thread;

/// Get the target triple for the host by calling into `rustc`.
pub(crate) async fn target_triple(root: &Path) -> Result<String> {
    let mut rustc_call = rustc()?.arg("-vV").spawn(CurrentHost::new(root.into()))?;
    let mut stdout = {
        let stdout = rustc_call
            .stdout()
            .ok_or_else(|| eyre!("no stdout for rustc call"))?;
        BufReader::new(stdout)
    };

    // In the background, run the rustc process.
    let rustc_runner = spawn(async move { rustc_call.exit().await });

    // Read lines from stdout.
    let mut line = String::new();
    while stdout.read_line(&mut line).await.is_ok() {
        // If the line is empty, break out.
        if line.is_empty() {
            break;
        }

        // If the line starts with "host: ", get the part after.
        if let Some(target) = line.strip_prefix("host: ") {
            // This is the target. Clean up the rustc process before stopping.
            drop(stdout);
            rustc_runner.await?;
            return Ok(target.trim().to_string());
        }

        line.clear();
    }

    Err(eyre!("unable to find 'host:' line in rustc output"))
}

/// Spawn a future onto the global executor.
pub(crate) fn spawn<F: Future + Send + 'static>(f: F) -> Task<F::Output>
where
    F::Output: Send + 'static,
{
    executor().spawn(f)
}

/// Run the executor alongside this future.
pub(crate) async fn run<F: Future>(f: F) -> F::Output {
    executor().run(f).await
}

fn executor() -> &'static Executor<'static> {
    static EXECUTOR: OnceCell<Executor<'static>> = OnceCell::new();

    EXECUTOR.get_or_init(|| {
        // Only use two executor threads.
        for i in 0..2 {
            thread::Builder::new()
                .name(format!("winit-test-runner-{i}"))
                .spawn(|| {
                    async_io::block_on(executor().run(pending::<()>()));
                })
                .expect("failed to spawn runner thread");
        }

        Executor::new()
    })
}
