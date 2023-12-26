// MIT/Apache2 License

//! Choose the proper host environment to run inside.

use super::DynEnvironment;
use crate::runner::Check;

use color_eyre::eyre::Result;
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
    let host = {
        let host = super::host::CurrentHost::new(root.to_path_buf());
        Arc::new(DynEnvironment::from_environment(host))
    };

    // Insert it into the list.
    let mut open_environments = open_environments.write().unwrap();
    open_environments.insert(key, host.clone());
    Ok(host)
}
