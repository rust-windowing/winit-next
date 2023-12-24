// MIT/Apache2 License

use crate::runner::command::{cargo_for_crate, run, rustfmt};
use crate::runner::util::spawn;
use crate::runner::Crate;

use futures_lite::prelude::*;
use futures_lite::stream;

use async_executor::Task;
use color_eyre::Result;
use tracing::Instrument;

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

const FMT_TIMEOUT: Duration = Duration::from_secs(30);
const CLIPPY_TIMEOUT: Duration = Duration::from_secs(5 * 60);

/// Run style tests on this workspace.
pub async fn style(root: &Path, crates: Vec<Crate>) -> Result<()> {
    let root: Arc<Path> = root.to_path_buf().into();

    // Spawn handles for style tasks.
    let handles: [Task<Result<()>>; 2] = [
        spawn({
            let root = root.clone();
            let span = tracing::info_span!("rustfmt");
            async move {
                rust_fmt(&root).await?;
                tracing::info!("rustfmt completed with no errors");
                Ok(())
            }
            .instrument(span)
        }),
        spawn({
            let root = root.clone();
            let span = tracing::info_span!("clippy");
            async move {
                rust_clippy(&root, &crates).await?;
                tracing::info!("clippy completed with no errors");
                Ok(())
            }
            .instrument(span)
        }),
    ];

    for handle in handles {
        handle.await?;
    }

    Ok(())
}

/// Run Rust formatting.
async fn rust_fmt(root: &Path) -> Result<()> {
    // Get all of the Rust files.
    let mut rust_files = files_with_extensions(root, "rs");

    // Create a command to run rustfmt.
    let mut fmt = rustfmt()?;
    fmt.args(["--edition", "2021", "--check"]);
    fmt.current_dir(root);
    while let Some(rust_file) = rust_files.next().await {
        fmt.arg(rust_file?);
    }
    run("rustfmt", &mut fmt, Some(FMT_TIMEOUT)).await?;

    Ok(())
}

/// Run clippy.
async fn rust_clippy(root: &Path, crates: &[Crate]) -> Result<()> {
    let command_runner = stream::iter(
        crates
            .iter()
            .flat_map(|crate_| cargo_for_crate(&["clippy"], crate_)),
    )
    .then(|command| async move {
        match command {
            Ok(mut command) => run("clippy", command.current_dir(root), Some(CLIPPY_TIMEOUT)).await,
            Err(err) => Err(err),
        }
    });
    futures_lite::pin!(command_runner);
    command_runner.try_for_each(|result| result).await?;

    Ok(())
}

/// Get all of the files in this namespace with this extension.
fn files_with_extensions(
    root: &Path,
    ext: impl AsRef<OsStr>,
) -> impl Stream<Item = Result<PathBuf>> + 'static {
    let root = root.to_path_buf();
    let ext = ext.as_ref().to_os_string();

    let walker = blocking::unblock(move || ignore::WalkBuilder::new(root).build());

    stream::once_future(walker)
        .flat_map(|walker| blocking::Unblock::with_capacity(16, walker))
        .filter(move |entry| {
            if let Ok(entry) = entry {
                entry.file_type().map(|f| f.is_file()) == Some(true)
                    && entry.path().extension() == Some(&*ext)
            } else {
                true
            }
        })
        .map(|result| match result {
            Ok(entry) => Ok(entry.into_path()),
            Err(err) => Err(err.into()),
        })
}
