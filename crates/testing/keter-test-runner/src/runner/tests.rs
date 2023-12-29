// MIT/Apache2 License

//! Run the test suite found in `keter-test`.

use color_eyre::eyre::Result;
use std::path::Path;

use crate::runner::{Crate, Check};

pub(crate) async fn tests(
    root: &Path,
    crates: Vec<Crate>,
) -> Result<()> {
    for crate_ in crates {

    }

    Ok(())
}
