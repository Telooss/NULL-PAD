use crate::paths::APP_DIR;
use std::io;
use std::path::Path;
use windows::core::{w, HSTRING, PCWSTR};
use windows::Win32::System::Registry::{
    RegDeleteKeyValueW, RegGetValueW, RegSetKeyValueW, HKEY_CURRENT_USER, REG_SZ, RRF_RT_REG_SZ,
};

const RUN_KEY: PCWSTR = w!(r"Software\Microsoft\Windows\CurrentVersion\Run");

pub fn is_autostart() -> bool {
    let name = HSTRING::from(APP_DIR);
    unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            RUN_KEY,
            &name,
            RRF_RT_REG_SZ,
            None,
            None,
            None,
        )
        .is_ok()
    }
}

pub fn set_autostart(enabled: bool) -> io::Result<()> {
    set_autostart_for(enabled, &std::env::current_exe()?)
}

/// Comme `set_autostart`, mais la valeur `Run` pointe vers `target` (l'interface lance le moteur).
pub fn set_autostart_for(enabled: bool, target: &Path) -> io::Result<()> {
    let name = HSTRING::from(APP_DIR);
    let err = if enabled {
        let exe = format!("\"{}\"", target.display());
        let data: Vec<u16> = exe.encode_utf16().chain(Some(0)).collect();
        unsafe {
            RegSetKeyValueW(
                HKEY_CURRENT_USER,
                RUN_KEY,
                &name,
                REG_SZ.0,
                Some(data.as_ptr().cast()),
                (data.len() * 2) as u32,
            )
        }
    } else {
        let e = unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, RUN_KEY, &name) };
        // ERROR_FILE_NOT_FOUND : deja desactive.
        if e.0 == 2 {
            return Ok(());
        }
        e
    };
    err.ok().map_err(io::Error::other)
}

/// Ancienne valeur `Run` "NULL-PAD" presente : la supprime et cree `nullctl` vers l'exe courant.
pub fn migrate_autostart() {
    let old = HSTRING::from(crate::paths::OLD_APP_DIR);
    // ERROR_FILE_NOT_FOUND (2) : pas d'ancienne valeur, le choix de l'utilisateur est garde.
    if unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, RUN_KEY, &old) }.0 == 0 {
        if let Err(e) = set_autostart(true) {
            tracing::error!("migration du demarrage auto impossible: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Restore(bool);
    impl Drop for Restore {
        fn drop(&mut self) {
            let _ = set_autostart(self.0);
        }
    }

    #[test]
    fn autostart_roundtrip() {
        let _restore = Restore(is_autostart());
        set_autostart(true).unwrap();
        assert!(is_autostart());
        set_autostart(false).unwrap();
        assert!(!is_autostart());
        set_autostart(false).unwrap();
    }
}
