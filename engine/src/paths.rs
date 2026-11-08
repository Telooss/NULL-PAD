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

/// Migre `base/NULL-PAD` vers `base/nullctl` quand l'ancienne config existe et pas la nouvelle.
/// Le test porte sur la config, pas sur le dossier : le logger peut deja avoir cree `nullctl/logs`.
pub fn migrate_app_dir(base: &Path) -> io::Result<bool> {
    let (old, new) = (base.join(OLD_APP_DIR), base.join(APP_DIR));
    if !old.join("config.json").exists() || new.join("config.json").exists() {
        return Ok(false);
    }
    if !new.exists() {
        std::fs::rename(old, new)?;
        return Ok(true);
    }
    for entry in std::fs::read_dir(&old)? {
        let entry = entry?;
        let dest = new.join(entry.file_name());
        if !dest.exists() {
            std::fs::rename(entry.path(), dest)?;
        }
    }
    // Echoue (sans effet) si l'ancien dossier garde des entrees, ex. un `logs` en double.
    let _ = std::fs::remove_dir(&old);
    Ok(true)
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

    fn old_with_config(base: &Path) {
        std::fs::create_dir_all(base.join(OLD_APP_DIR).join("sounds")).unwrap();
        std::fs::write(base.join(OLD_APP_DIR).join("config.json"), "x").unwrap();
    }

    #[test]
    fn renames_old_dir_when_alone() {
        let base = tmp("alone");
        old_with_config(&base);
        assert!(migrate_app_dir(&base).unwrap());
        assert!(!base.join(OLD_APP_DIR).exists());
        assert!(base.join(APP_DIR).join("config.json").exists());
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn merges_into_new_dir_holding_only_logs() {
        let base = tmp("logs");
        old_with_config(&base);
        std::fs::create_dir_all(base.join(APP_DIR).join("logs")).unwrap();
        assert!(migrate_app_dir(&base).unwrap());
        assert!(base.join(APP_DIR).join("config.json").exists());
        assert!(base.join(APP_DIR).join("sounds").is_dir());
        assert!(base.join(APP_DIR).join("logs").is_dir());
        assert!(!base.join(OLD_APP_DIR).exists());
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn keeps_both_when_new_has_config() {
        let base = tmp("both");
        old_with_config(&base);
        std::fs::create_dir(base.join(APP_DIR)).unwrap();
        std::fs::write(base.join(APP_DIR).join("config.json"), "y").unwrap();
        assert!(!migrate_app_dir(&base).unwrap());
        assert!(base.join(OLD_APP_DIR).join("config.json").exists());
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn noop_when_old_missing() {
        let base = tmp("none");
        assert!(!migrate_app_dir(&base).unwrap());
        assert!(!base.join(APP_DIR).exists());
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn old_dir_survives_when_an_entry_cannot_move() {
        let base = tmp("leftover");
        old_with_config(&base);
        // Meme nom des deux cotes : l'entree reste dans l'ancien dossier, qui n'est pas supprime.
        std::fs::create_dir_all(base.join(APP_DIR).join("sounds")).unwrap();
        assert!(migrate_app_dir(&base).unwrap());
        assert!(base.join(APP_DIR).join("config.json").exists());
        assert!(base.join(OLD_APP_DIR).join("sounds").is_dir());
        let _ = std::fs::remove_dir_all(base);
    }
}
