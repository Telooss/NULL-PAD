//! Config sur le disque : chargement, validation (regles du moteur) et ecriture atomique.
use crate::native;
use engine::config::Config;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

pub fn load(path: &Path) -> Result<Config, String> {
    engine::config::load(path).map_err(|e| e.to_string())
}

/// Regles du moteur (`config::parse`), plus : aucune liaison sur une touche native du firmware.
pub fn validate(json: &str) -> Result<(), String> {
    let c = engine::config::parse(json).map_err(|e| e.to_string())?;
    for p in &c.profiles {
        if let Some(b) = p
            .bindings
            .iter()
            .find(|b| native::is_native(native::KEYMAP, p.id, b.input.0))
        {
            return Err(format!(
                "touche native du firmware : {} (profil {}), modifiable plus tard depuis le pad",
                b.input.name(),
                p.id
            ));
        }
    }
    Ok(())
}

/// Valide, ecrit `<nom>.tmp` a cote, le vide sur le disque, puis le renomme sur `path`.
pub fn save_atomic(path: &Path, json: &str) -> Result<(), String> {
    validate(json)?;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    let write = || -> std::io::Result<()> {
        let mut f = File::create(&tmp)?;
        f.write_all(json.as_bytes())?;
        f.sync_all()?;
        drop(f);
        fs::rename(&tmp, path) // remplace atomiquement sur le meme volume
    };
    write().map_err(|e| {
        let _ = fs::remove_file(&tmp);
        format!("ecriture de {}: {e}", path.display())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine::config::default_config;

    fn temp_dir(tag: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("nullctl-ui-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        d
    }

    fn valid_json() -> String {
        serde_json::to_string_pretty(&default_config()).unwrap()
    }

    #[test]
    fn save_atomic_never_leaves_partial() {
        let dir = temp_dir("save");
        let path = dir.join("config.json");
        let good = valid_json();
        save_atomic(&path, &good).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), good);
        assert!(save_atomic(&path, "{\"version\":1").is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), good);
        assert!(!dir.join("config.json.tmp").exists());
        assert_eq!(load(&path).unwrap(), default_config());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn validate_rejects_bad_json_and_rules() {
        assert!(validate(&valid_json()).is_ok());
        assert!(validate("pas du json").unwrap_err().starts_with("json:"));
        let mut c = default_config();
        c.profiles[0].tag = "trop long".into();
        let e = validate(&serde_json::to_string(&c).unwrap()).unwrap_err();
        assert!(e.starts_with("config invalide:"), "{e}");
    }

    #[test]
    fn validate_rejects_native_keys() {
        let with = |profile: u8, input: &str| {
            let mut c = default_config();
            c.profiles[0].id = profile;
            let mut b = c.profiles[0].bindings[2].clone(); // MicMute sur appui
            b.input = engine::codes::InputId::from_name(input).unwrap();
            c.profiles[0].bindings.push(b);
            validate(&serde_json::to_string(&c).unwrap())
        };
        assert!(with(1, "K04").unwrap_err().contains("K04"));
        assert!(with(2, "M1").unwrap_err().contains("M1"));
        assert!(with(1, "K11").is_ok());
        assert!(with(2, "K04").is_ok());
    }
}
