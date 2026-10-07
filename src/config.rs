use crate::constants::APP_NAME;
use color_eyre::eyre::{Result, eyre};
use directories::BaseDirs;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

pub struct Config {
    pub log_dir: PathBuf,
    pub data_dir: PathBuf,
}

impl Config {
    pub fn init() -> Result<Self> {
        // default logs dir
        let log_dir = get_default_state_dir()?.join("logs");
        fs::create_dir_all(&log_dir)?;

        // default data dir
        let data_dir = get_default_state_dir()?.join("data");
        fs::create_dir_all(&data_dir)?;

        Ok(Self { log_dir, data_dir })
    }
}

/// XDG state dir on every platform: `$XDG_STATE_HOME/timr-tui`,
/// falling back to `~/.local/state/timr-tui`.
fn get_default_state_dir() -> Result<PathBuf> {
    let base = BaseDirs::new().ok_or_else(|| eyre!("Failed to get home directory"))?;
    Ok(state_dir(
        std::env::var_os("XDG_STATE_HOME"),
        base.home_dir(),
    ))
}

fn state_dir(xdg_state_home: Option<OsString>, home: &Path) -> PathBuf {
    xdg_state_home
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".local").join("state"))
        .join(APP_NAME)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_dir() {
        let home = Path::new("/home/me");
        assert_eq!(
            state_dir(None, home),
            PathBuf::from("/home/me/.local/state/timr-tui")
        );
        assert_eq!(
            state_dir(Some("".into()), home),
            PathBuf::from("/home/me/.local/state/timr-tui"),
            "empty XDG_STATE_HOME is treated as unset"
        );
        assert_eq!(
            state_dir(Some("/tmp/state".into()), home),
            PathBuf::from("/tmp/state/timr-tui")
        );
    }
}
