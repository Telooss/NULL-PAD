//! Cache de la dernière lecture du pad (`pad-cache.json`), pour l'édition hors ligne.
use super::model::PadModel;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Cache {
    pub serial: String,
    /// Secondes Unix.
    pub read_at: u64,
    pub model: PadModel,
    pub max_name_len: usize,
}

pub fn path() -> PathBuf {
    engine::paths::config_path().with_file_name("pad-cache.json")
}

/// `None` si le fichier manque, est corrompu, ou si `serial` est donné et diffère.
pub fn load(path: &Path, serial: Option<&str>) -> Option<Cache> {
    let c: Cache = serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()?;
    serial.is_none_or(|s| s == c.serial).then_some(c)
}

pub fn save(path: &Path, cache: &Cache) -> Result<(), String> {
    let json = serde_json::to_string(cache).map_err(|e| e.to_string())?;
    crate::store::write_atomic(path, &json)
}
