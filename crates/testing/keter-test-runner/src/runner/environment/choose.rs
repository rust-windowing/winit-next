// MIT/Apache2 License

//! Choose the proper host environment to run inside.

use super::{DynEnvironment, Environment};
use crate::runner::util::target_triple;
use crate::runner::Check;

use color_eyre::eyre::{bail, Result};
use once_cell::sync::OnceCell;

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, RwLock};

static OPEN_ENVIRONMENTS: OnceCell<RwLock<HashMap<CheckKey, Arc<DynEnvironment>>>> =
    OnceCell::new();

#[derive(Debug, PartialEq, Eq, Hash)]
struct CheckKey {
    target_triple: String,
    host_env: Option<String>,
}

#[allow(clippy::never_loop)]
pub(crate) async fn choose(root: &Path, check: &Check) -> Result<Arc<DynEnvironment>> {
    // Figure out our current check key.
    let key = CheckKey {
        target_triple: check.target_triple.clone(),
        host_env: check.host_env.clone(),
    };

    // See if we have an environment matching this.
    let open_environments = OPEN_ENVIRONMENTS.get_or_init(|| RwLock::new(HashMap::new()));
    {
        let open_environments = open_environments.read().unwrap();
        if let Some(host) = open_environments.get(&key) {
            return Ok(host.clone());
        }
    }

    // Otherwise, we need to choose it.
    // TODO: Other environments.
    let host = Arc::new(loop {
        // Get the current target triple.
        let host_target = target_triple(root).await?;

        // If the triple is the same as our desired triple, use the current host environment.
        if host_target == check.target_triple {
            let host = super::host::CurrentHost::new(root.to_path_buf());
            break DynEnvironment::from_environment(host);
        }

        // If the host is Linux and the target is Android, use the Android runner.
        if host_target.contains("linux") && check.target_triple.contains("android") {
            let host = super::android::AndroidEnvironment::new(root.to_path_buf());
            break DynEnvironment::from_environment(host);
        }

        bail!(
            "cannot find compatible environment for host {host_target} and target {}",
            &check.target_triple
        )
    });

    // Insert it into the list.
    let mut open_environments = open_environments.write().unwrap();
    open_environments.insert(key, host.clone());
    Ok(host)
}

pub(crate) async fn cleanup() -> Result<()> {
    let mut open_environments = match OPEN_ENVIRONMENTS.get() {
        None => return Ok(()),
        Some(oe) => oe.write().unwrap(),
    };

    for (_, host) in open_environments.drain() {
        host.cleanup().await?;
    }

    Ok(())
}
