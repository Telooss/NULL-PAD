//! Reglages : sorties audio, VB-Cable, demarrage avec Windows (moteur a cote de l'interface).
use crate::pipes::ENGINE_EXE;
use std::path::{Path, PathBuf};

/// `nullctl-engine.exe` dans le dossier de l'exe `current`.
pub fn engine_exe(current: &Path) -> PathBuf {
    current.with_file_name(ENGINE_EXE)
}

pub fn set_autostart(enabled: bool) -> Result<(), String> {
    let exe = engine_exe(&std::env::current_exe().map_err(|e| e.to_string())?);
    if enabled && !exe.exists() {
        return Err(format!("{ENGINE_EXE} introuvable dans {}", exe.parent().unwrap_or(&exe).display()));
    }
    engine::autostart::set_autostart_for(enabled, &exe).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_exe_sits_next_to_ui_exe() {
        let p = engine_exe(Path::new(r"C:\Apps\nullctl\nullctl.exe"));
        assert_eq!(p, PathBuf::from(r"C:\Apps\nullctl\nullctl-engine.exe"));
    }
}
