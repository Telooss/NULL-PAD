use std::path::PathBuf;

pub const APP_DIR: &str = "macropad";

pub fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_default()
        .join(APP_DIR)
        .join("config.json")
}
