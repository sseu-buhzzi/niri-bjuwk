use std::{
    env, fmt,
    path::{Path, PathBuf},
    sync::OnceLock,
};

use chrono::{DateTime, TimeZone};

pub struct PathManager {
    state_dir_path: PathBuf,
    config_dir_path: PathBuf,
}

impl PathManager {
    pub fn get() -> &'static Self {
        static INSTANCE: OnceLock<PathManager> = OnceLock::new();

        INSTANCE.get_or_init(|| Self {
            state_dir_path: state_dir(),
            config_dir_path: config_dir(),
        })
    }

    pub fn state_dir_path(&self) -> &Path {
        &self.state_dir_path
    }

    pub fn config_dir_path(&self) -> &Path {
        &self.config_dir_path
    }

    pub fn snapshot_store_dir_path(&self) -> PathBuf {
        self.state_dir_path.join("snapshot.d")
    }

    pub fn snapshot_path<Tz>(&self, date_time: DateTime<Tz>) -> PathBuf
    where
        Tz: TimeZone,
        Tz::Offset: fmt::Display,
    {
        let name = date_time.format("%Y%m%d_%H%M%S.json").to_string();
        self.snapshot_store_dir_path().join(name)
    }

    pub fn snapshot_link_path(&self) -> PathBuf {
        self.state_dir_path.join("snapshot.json")
    }

    pub fn match_config_path(&self) -> PathBuf {
        self.config_dir_path.join("match.yaml")
    }
}

fn state_dir() -> PathBuf {
    env_xdg_state_home()
        .or_else(|| env_home().map(|p| p.join(".local/state")))
        .unwrap_or_else(|| PathBuf::from("/var/tmp"))
        .join(env!("CARGO_PKG_NAME"))
}

fn config_dir() -> PathBuf {
    env_xdg_config_home()
        .or_else(|| env_home().map(|p| p.join(".config")))
        .unwrap_or_else(|| PathBuf::from("/etc"))
        .join(env!("CARGO_PKG_NAME"))
}

fn env_xdg_state_home() -> Option<PathBuf> {
    env::var_os("XDG_STATE_HOME")
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
}

fn env_xdg_config_home() -> Option<PathBuf> {
    env::var_os("XDG_CONFIG_HOME")
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
}

fn env_home() -> Option<PathBuf> {
    env::var_os("HOME")
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
}
