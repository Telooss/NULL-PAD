use std::io;
use std::path::{Path, PathBuf};

pub const APP_DIR: &str = "nullctl";
pub const OLD_APP_DIR: &str = "NULL-PAD";

pub fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_default()
        .join(APP_DIR)
        .join("config.json")
}

/// Renomme `base/NULL-PAD` en `base/nullctl` si seul l'ancien existe.
pub fn migrate_app_dir(base: &Path) -> io::Result<bool> {
    let (old, new) = (base.join(OLD_APP_DIR), base.join(APP_DIR));
    if old.exists() && !new.exists() {
        std::fs::rename(old, new)?;
        return Ok(true);
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("nullctl-paths-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn renames_old_dir_when_alone() {
        let base = tmp("alone");
        std::fs::create_dir(base.join(OLD_APP_DIR)).unwrap();
        std::fs::write(base.join(OLD_APP_DIR).join("config.json"), "x").unwrap();
        assert!(migrate_app_dir(&base).unwrap());
        assert!(!base.join(OLD_APP_DIR).exists());
        assert!(base.join(APP_DIR).join("config.json").exists());
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn keeps_both_when_new_exists() {
        let base = tmp("both");
        std::fs::create_dir(base.join(OLD_APP_DIR)).unwrap();
        std::fs::create_dir(base.join(APP_DIR)).unwrap();
        assert!(!migrate_app_dir(&base).unwrap());
        assert!(base.join(OLD_APP_DIR).exists());
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn noop_when_none_exists() {
        let base = tmp("none");
        assert!(!migrate_app_dir(&base).unwrap());
        assert!(!base.join(APP_DIR).exists());
        let _ = std::fs::remove_dir_all(base);
    }
}
