use std::{fs, path::Path};

use niri_ipc::{Window, Workspace};
use serde::{Deserialize, Serialize};

use crate::error::{BjuwkError, BjuwkResult, IoContextExt};

#[derive(Serialize, Deserialize)]
pub struct Snapshot {
    pub version: String,
    pub workspaces: Vec<Workspace>,
    pub windows: Vec<Window>,
}

impl Snapshot {
    pub fn new(workspaces: Vec<Workspace>, windows: Vec<Window>) -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            workspaces,
            windows,
        }
    }

    pub fn save(&self, path: &Path) -> BjuwkResult<()> {
        let bytes = serde_json::to_vec(self)?;
        fs::write(path, bytes).context(format!("Snapshot::save {}", path.display()))?;
        Ok(())
    }

    pub fn load(path: &Path) -> BjuwkResult<Self> {
        let bytes = fs::read(path).context(format!("Snapshot::load {}", path.display()))?;
        let snap: Self = serde_json::from_slice(&bytes)?;
        // TODO: allow older versions.
        if snap.version != env!("CARGO_PKG_VERSION") {
            return Err(BjuwkError::InvalidSnapshot(format!(
                "unknown snapshot version {}",
                snap.version,
            )));
        }
        Ok(snap)
    }
}
