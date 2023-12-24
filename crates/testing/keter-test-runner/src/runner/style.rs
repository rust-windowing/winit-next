// MIT/Apache2 License

use crate::runner::command::{cargo, rustfmt};

use color_eyre::eyre::bail;
use color_eyre::Result;

use futures_lite::prelude::*;
use futures_lite::stream;

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

/// Run style tests on this workspace.
pub async fn style(root: &Path) -> Result<()> {
    // Get all of the Rust files.
    let mut rust_files = files_with_extensions(root, "rs");

    // Create a command to run rustfmt.
    let mut fmt = rustfmt();
    fmt.args(["--edition", "2021", "--check"]);
    while let Some(rust_file) = rust_files.next().await {
        fmt.arg(rust_file?);
    }
    if !fmt.status().await?.success() {
        bail!("rustfmt did not return a positive error code");
    }

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
