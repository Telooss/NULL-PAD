//! Touches natives : lues dans la keymap du firmware (ce que le moteur ne voit jamais).
use engine::codes::InputId;
use std::collections::BTreeMap;

pub const KEYMAP: &str = include_str!("../../firmware/null_pad.keymap");

/// Liaisons de chaque couche, dans l'ordre des positions (= InputId) : `kp KP_N7`, `np_00`…
fn layers(keymap: &str) -> Vec<Vec<String>> {
    let mut out = Vec::new();
    let mut lines = keymap.lines();
    // Les couches ecrivent `bindings = <` seul sur sa ligne ; macros et molettes tiennent sur une ligne.
    while lines.any(|l| l.trim() == "bindings = <") {
        let body: Vec<&str> = lines.by_ref().take_while(|l| l.trim() != ">;").collect();
        let body = body.join(" ");
        out.push(
            body.split('&')
                .map(|b| b.split_whitespace().collect::<Vec<_>>().join(" "))
                .filter(|b| !b.is_empty())
                .collect(),
        );
    }
    out
}

/// Legende d'une liaison native ; None pour une action envoyee au moteur (`ACT(…)`).
fn legend(binding: &str) -> Option<String> {
    if binding.contains("ACT(") {
        return None;
    }
    let code = binding.strip_prefix("kp ").unwrap_or(binding);
    let known = match code {
        "KP_SLASH" => "/",
        "KP_MULTIPLY" => "*",
        "KP_MINUS" => "−",
        "KP_PLUS" => "+",
        "KP_ENTER" => "ENTRÉE",
        "KP_DOT" => ".",
        "KP_NUMLOCK" => "VERR NUM",
        "np_00" => "00",
        "C_VOL_UP" => "VOL+",
        "C_VOL_DN" => "VOL−",
        "C_MUTE" => "MUET",
        _ => {
            // KP_N7 -> 7, np_p1 -> P1, autre code -> son nom.
            let label = code
                .strip_prefix("KP_N")
                .or_else(|| code.strip_prefix("np_"))
                .unwrap_or(code);
            return Some(label.to_uppercase());
        }
    };
    Some(known.to_string())
}

/// Legende de chaque entree native de la couche 0 (`K00` -> « VERR NUM »).
pub fn native_keys(keymap: &str) -> BTreeMap<String, String> {
    let layers = layers(keymap);
    let Some(layer) = layers.first() else {
        return BTreeMap::new();
    };
    layer
        .iter()
        .take(29) // InputId 0..=28
        .enumerate()
        .filter_map(|(i, b)| Some((InputId(i as u8).name().to_string(), legend(b)?)))
        .collect()
}

/// Vrai si l'entree `input` est native dans le profil `profile` (couche `profile - 1`).
pub fn is_native(keymap: &str, profile: u8, input: u8) -> bool {
    let layers = layers(keymap);
    let binding = (profile as usize)
        .checked_sub(1)
        .and_then(|l| layers.get(l))
        .and_then(|l| l.get(input as usize));
    binding.is_some_and(|b| legend(b).is_some())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_keys_from_real_keymap() {
        let m = native_keys(KEYMAP);
        assert_eq!(m["K00"], "VERR NUM");
        assert_eq!(m["K04"], "7");
        assert_eq!(m["K16"], "0");
        assert_eq!(m["K17"], "00");
        assert_eq!(m["K03"], "−");
        assert_eq!(m["K15"], "ENTRÉE");
        assert!(!m.contains_key("K11"));
        assert!(!m.contains_key("K19"));
        assert_eq!(m["M1"], "P1");
        assert_eq!(m["M3"], "P3");
        assert_eq!(m["ENC_L_CW"], "VOL+");
        assert_eq!(m["ENC_L_CCW"], "VOL−");
        assert_eq!(m["ENC_L_PRESS"], "MUET");
        assert!(!m.keys().any(|k| k.starts_with("ENC_R")));
        assert_eq!(m.len(), 18 + 3 + 3);
    }

    #[test]
    fn natives_per_profile() {
        assert!(is_native(KEYMAP, 1, 4));
        assert!(!is_native(KEYMAP, 1, 11));
        assert!(!is_native(KEYMAP, 2, 4));
        assert!(is_native(KEYMAP, 2, 20)); // M1 sur toutes les couches
        assert!(is_native(KEYMAP, 8, 25)); // clic molette gauche
        assert!(!is_native(KEYMAP, 8, 28));
        assert!(!is_native(KEYMAP, 9, 20)); // couche absente
    }
}
