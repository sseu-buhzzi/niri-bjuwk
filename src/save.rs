use std::{
    fs,
    os::unix,
    path::{Path, PathBuf},
};

use chrono::Local;

use crate::{
    error::{BjuwkResult, IoContextExt},
    niri_ipc_wrapper::NiriIpcWrapper,
    path_util::PathManager,
    snapshot::Snapshot,
};

pub fn execute(snapshot_path: Option<PathBuf>) -> BjuwkResult<()> {
    let store_path = write_store()?;
    link_store(snapshot_path, &store_path)?;

    Ok(())
}

fn write_store() -> BjuwkResult<PathBuf> {
    let pm = PathManager::get();
    let store_path = pm.snapshot_path(Local::now());

    let niw = NiriIpcWrapper::connect(false)?;
    let (workspaces, windows) = niw.receive_workspaces_and_windows()?;

    let snap = Snapshot::new(workspaces, windows);
    let store_dir_path = pm.snapshot_store_dir_path();
    fs::create_dir_all(&store_dir_path)
        .context(format!("create_dir_all({})", store_dir_path.display()))?;
    snap.save(&store_path)?;

    println!(
        "Saved {} workspaces and {} windows to {}",
        snap.workspaces.len(),
        snap.windows.len(),
        store_path.display()
    );
    Ok(store_path)
}

fn link_store(link_path: Option<PathBuf>, snapshot_path: &Path) -> BjuwkResult<()> {
    let pm = PathManager::get();
    let link_path = link_path.unwrap_or_else(|| pm.snapshot_link_path());
    let tmp_path = link_path.with_added_extension("tmp");
    let snapshot_path = snapshot_path
        .strip_prefix(pm.state_dir_path())
        .unwrap_or(snapshot_path);

    unix::fs::symlink(snapshot_path, &tmp_path).context(format!(
        "symlink({}, {})",
        snapshot_path.display(),
        tmp_path.display()
    ))?;
    scopeguard::defer! {
        let _ = fs::remove_file(&tmp_path);
    }
    fs::rename(&tmp_path, &link_path).context(format!(
        "rename({}, {})",
        tmp_path.display(),
        link_path.display()
    ))?;

    Ok(())
}
