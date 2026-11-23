//! Touches HID : noms français, catalogue pour l'interface, modificateurs implicites ZMK.
//!
//! Codes recopiés de ZMK v0.3.0 (app/include/dt-bindings/zmk/{keys,hid_usage,modifiers}.h).
//! Un usage ZMK = (page << 16) | id ; les bits 24..31 portent les modificateurs implicites
//! (`LC(C)` = MOD_LCTL << 24 | C) : 0x01 LCtrl, 0x02 LShift, 0x04 LAlt, 0x08 LGui, puis 0x10..0x80 côté droit.
use serde::Serialize;

const KEY: u32 = 0x07;
const CONSUMER: u32 = 0x0C;

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
pub struct KeyChoice {
    pub usage: u32,
    pub label: String,
    pub group: &'static str,
}

const fn u(page: u32, id: u32) -> u32 {
    (page << 16) | id
}

/// Touches nommées à la main : (usage, libellé, groupe).
const NAMED: &[(u32, &str, &str)] = &[
    (u(KEY, 0x28), "Entrée", "Clavier"),
    (u(KEY, 0x29), "Échap", "Clavier"),
    (u(KEY, 0x2A), "Retour arrière", "Clavier"),
    (u(KEY, 0x2B), "Tab", "Clavier"),
    (u(KEY, 0x2C), "Espace", "Clavier"),
    (u(KEY, 0x39), "Verr Maj", "Clavier"),
    (u(KEY, 0x46), "Impr écran", "Clavier"),
    (u(KEY, 0x47), "Arrêt défil", "Clavier"),
    (u(KEY, 0x48), "Pause", "Clavier"),
    (u(KEY, 0x49), "Inser", "Clavier"),
    (u(KEY, 0x4A), "Début", "Clavier"),
    (u(KEY, 0x4B), "Page préc.", "Clavier"),
    (u(KEY, 0x4C), "Suppr", "Clavier"),
    (u(KEY, 0x4D), "Fin", "Clavier"),
    (u(KEY, 0x4E), "Page suiv.", "Clavier"),
    (u(KEY, 0x4F), "Droite", "Clavier"),
    (u(KEY, 0x50), "Gauche", "Clavier"),
    (u(KEY, 0x51), "Bas", "Clavier"),
    (u(KEY, 0x52), "Haut", "Clavier"),
    (u(KEY, 0x65), "Menu", "Clavier"),
    (u(KEY, 0x53), "Verr Num", "Pavé numérique"),
    (u(KEY, 0x54), "/", "Pavé numérique"),
    (u(KEY, 0x55), "*", "Pavé numérique"),
    (u(KEY, 0x56), "-", "Pavé numérique"),
    (u(KEY, 0x57), "+", "Pavé numérique"),
    (u(KEY, 0x58), "Entrée", "Pavé numérique"),
    (u(KEY, 0x62), "0", "Pavé numérique"),
    (u(KEY, 0x63), ".", "Pavé numérique"),
    (u(KEY, 0x67), "=", "Pavé numérique"),
    (u(CONSUMER, 0xB0), "Lecture", "Média et volume"),
    (u(CONSUMER, 0xB1), "Pause", "Média et volume"),
    (u(CONSUMER, 0xCD), "Lecture/Pause", "Média et volume"),
    (u(CONSUMER, 0xB5), "Piste suivante", "Média et volume"),
    (u(CONSUMER, 0xB6), "Piste précédente", "Média et volume"),
    (u(CONSUMER, 0xB7), "Stop", "Média et volume"),
    (u(CONSUMER, 0xE9), "Volume +", "Média et volume"),
    (u(CONSUMER, 0xEA), "Volume −", "Média et volume"),
    (u(CONSUMER, 0xE2), "Muet", "Média et volume"),
    (u(CONSUMER, 0x6F), "Luminosité +", "Média et volume"),
    (u(CONSUMER, 0x70), "Luminosité −", "Média et volume"),
    (u(KEY, 0xE0), "Ctrl gauche", "Modificateurs"),
    (u(KEY, 0xE1), "Maj gauche", "Modificateurs"),
    (u(KEY, 0xE2), "Alt gauche", "Modificateurs"),
    (u(KEY, 0xE3), "Win gauche", "Modificateurs"),
    (u(KEY, 0xE4), "Ctrl droite", "Modificateurs"),
    (u(KEY, 0xE5), "Maj droite", "Modificateurs"),
    (u(KEY, 0xE6), "Alt droite", "Modificateurs"),
    (u(KEY, 0xE7), "Win droite", "Modificateurs"),
];

/// Touches à caractère : (id HID, scancode jeu 1, caractère en QWERTY US).
/// Le pad envoie une position, pas un caractère : en AZERTY, l'id 0x04 (« A » US) tape « q ».
fn chars() -> Vec<(u32, u32, char)> {
    const LETTER_SC: [u32; 26] = [
        0x1E, 0x30, 0x2E, 0x20, 0x12, 0x21, 0x22, 0x23, 0x17, 0x24, 0x25, 0x26, 0x32, 0x31, 0x18,
        0x19, 0x10, 0x13, 0x1F, 0x14, 0x16, 0x2F, 0x11, 0x2D, 0x15, 0x2C,
    ];
    let mut v: Vec<_> = (0..26u32)
        .map(|i| (4 + i, LETTER_SC[i as usize], char::from(b'A' + i as u8)))
        .collect();
    v.extend((0..10u32).map(|i| (0x1E + i, 2 + i, char::from(b'0' + ((i + 1) % 10) as u8))));
    v.extend([
        (0x2D, 0x0C, '-'),
        (0x2E, 0x0D, '='),
        (0x2F, 0x1A, '['),
        (0x30, 0x1B, ']'),
        (0x31, 0x2B, '\\'),
        (0x33, 0x27, ';'),
        (0x34, 0x28, '\''),
        (0x35, 0x29, '`'),
        (0x36, 0x33, ','),
        (0x37, 0x34, '.'),
        (0x38, 0x35, '/'),
        (0x64, 0x56, '<'), // touche ISO à gauche de W en AZERTY
    ]);
    v
}

/// Caractère tapé sans modificateur par le scancode `sc` dans la disposition Windows active.
fn layout_char(sc: u32) -> Option<char> {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        GetKeyboardLayout, MapVirtualKeyExW, MAPVK_VK_TO_CHAR, MAPVK_VSC_TO_VK,
    };
    // SAFETY : appels sans pointeur ; 0 si la touche n'existe pas dans la disposition.
    let ch = unsafe {
        let hkl = GetKeyboardLayout(0);
        let vk = MapVirtualKeyExW(sc, MAPVK_VSC_TO_VK, Some(hkl));
        MapVirtualKeyExW(vk, MAPVK_VK_TO_CHAR, Some(hkl)) & 0x7FFF // bit 15 = touche morte
    };
    char::from_u32(ch).filter(|c| !c.is_control())
}

/// Toutes les touches sans modificateur, dans l'ordre d'affichage.
fn all() -> Vec<KeyChoice> {
    all_with(layout_char)
}

fn all_with(layout: impl Fn(u32) -> Option<char>) -> Vec<KeyChoice> {
    let mut v: Vec<KeyChoice> = chars()
        .into_iter()
        .map(|(id, sc, us)| KeyChoice {
            usage: u(KEY, id),
            label: layout(sc).unwrap_or(us).to_string(),
            group: "Clavier",
        })
        .collect();
    // Lettres A–Z d'abord, dans l'ordre alphabétique ; le reste garde l'ordre du clavier.
    v.sort_by_key(|k| {
        let mut cs = k.label.chars();
        let letter = cs.next().filter(|c| c.is_ascii_uppercase() && cs.next().is_none());
        (letter.is_none(), letter)
    });
    let mut add = |usage, label: String, group| {
        v.push(KeyChoice {
            usage,
            label,
            group,
        })
    };
    for i in 0..12u32 {
        add(u(KEY, 0x3A + i), format!("F{}", i + 1), "Clavier");
    }
    for i in 0..12u32 {
        add(u(KEY, 0x68 + i), format!("F{}", i + 13), "Clavier");
    }
    // Pavé : 1..9 en 0x59..0x61.
    for i in 0..9u32 {
        add(u(KEY, 0x59 + i), (i + 1).to_string(), "Pavé numérique");
    }
    for &(usage, label, group) in NAMED {
        add(usage, label.to_string(), group);
    }
    v
}

/// Catalogue des touches proposées par l'interface (sans combinaisons : voir `with_mods`).
pub fn catalog() -> Vec<KeyChoice> {
    all()
}

/// Ajoute les modificateurs implicites (côté gauche) à un usage.
#[allow(dead_code)] // réservé au sélecteur de modificateurs
pub fn with_mods(usage: u32, ctrl: bool, shift: bool, alt: bool, win: bool) -> u32 {
    let m = u32::from(ctrl) | u32::from(shift) << 1 | u32::from(alt) << 2 | u32::from(win) << 3;
    usage | m << 24
}

/// Libellé d'un usage, modificateurs implicites inclus ("Ctrl+Maj+C"). `None` si inconnu.
pub fn name(usage: u32) -> Option<String> {
    let base = usage & 0x00FF_FFFF;
    // Le clavier et le pavé partagent "1".."9" : on cherche sur l'usage exact.
    let label = all().into_iter().find(|k| k.usage == base)?.label;
    const MODS: [&str; 8] = [
        "Ctrl", "Maj", "Alt", "Win", "Ctrl D", "Maj D", "Alt D", "Win D",
    ];
    let mods = usage >> 24;
    let mut out = String::new();
    for (i, m) in MODS.iter().enumerate() {
        if mods & (1 << i) != 0 {
            out.push_str(m);
            out.push('+');
        }
    }
    Some(out + &label)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names() {
        assert_eq!(name(u(KEY, 0x5F)).as_deref(), Some("7"));
        assert_eq!(name(u(CONSUMER, 0xE9)).as_deref(), Some("Volume +"));
        assert_eq!(
            name(with_mods(u(KEY, 0x06), true, false, false, false)).as_deref(),
            Some("Ctrl+C")
        );
        assert_eq!(
            name(with_mods(u(KEY, 0x06), true, true, false, true)).as_deref(),
            Some("Ctrl+Maj+Win+C")
        );
        assert_eq!(name(u(KEY, 0x2C)).as_deref(), Some("Espace"));
        assert_eq!(name(u(KEY, 0x01)), None);
    }

    #[test]
    fn labels_follow_layout() {
        // Faux AZERTY : A et Q échangés, 1 tape « & ».
        let azerty = |sc| match sc {
            0x10 => Some('A'),
            0x1E => Some('Q'),
            0x02 => Some('&'),
            _ => None,
        };
        let c = all_with(azerty);
        let label = |id| &c.iter().find(|k| k.usage == u(KEY, id)).unwrap().label;
        assert_eq!(label(0x14), "A");
        assert_eq!(label(0x04), "Q");
        assert_eq!(label(0x1E), "&");
        assert_eq!(label(0x05), "B"); // inconnu : secours QWERTY
        assert_eq!(c[0].label, "A");
        assert_eq!(c[0].usage, u(KEY, 0x14));
    }

    #[test]
    fn catalog_has_no_duplicate_usage_and_all_groups() {
        let c = catalog();
        let mut us: Vec<_> = c.iter().map(|k| k.usage).collect();
        us.sort();
        us.dedup();
        assert_eq!(us.len(), c.len());
        for g in [
            "Clavier",
            "Pavé numérique",
            "Média et volume",
            "Modificateurs",
        ] {
            assert!(c.iter().any(|k| k.group == g), "{g}");
        }
    }
}
