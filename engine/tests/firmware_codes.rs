//! Verifie que l'en-tete du firmware produit exactement les codes du moteur.
use engine::codes::{encode_action, encode_profile, InputId};
use std::collections::HashMap;
use std::path::Path;

/// 29 = nombre d'entrees du moteur; les noms viennent de `InputId::name`.
const N_INPUTS: u8 = 29;

fn names() -> Vec<&'static str> {
    (0..N_INPUTS).map(|i| InputId(i).name()).collect()
}

/// `#define NOM valeur` simples (decimal ou 0x); les macros a parametres sont ignorees.
fn defines() -> HashMap<String, u32> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../firmware/null_pad_codes.h");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let mut out = HashMap::new();
    for line in text.lines() {
        let mut it = line.split_whitespace();
        if it.next() != Some("#define") {
            continue;
        }
        let (Some(name), Some(val)) = (it.next(), it.next()) else { continue };
        let parsed = match val.strip_prefix("0x") {
            Some(h) => u32::from_str_radix(h, 16),
            None => val.parse(),
        };
        if let Ok(v) = parsed {
            out.insert(name.to_string(), v);
        }
    }
    out
}

fn get(d: &HashMap<String, u32>, name: &str) -> u32 {
    *d.get(name).unwrap_or_else(|| panic!("#define {name} manquant dans null_pad_codes.h"))
}

#[test]
fn action_formula_matches_engine() {
    let d = defines();
    let (base, stride) = (get(&d, "NP_ACTION_BASE"), get(&d, "NP_PROFILE_STRIDE"));
    for p in 1..=8u32 {
        for n in names() {
            let i = get(&d, &format!("IN_{n}"));
            let want = encode_action(p as u8, InputId(i as u8)) as u32;
            assert_eq!(base + (p - 1) * stride + i, want, "profil {p} entree {n}");
        }
    }
}

#[test]
fn announce_matches_engine() {
    let d = defines();
    let base = get(&d, "NP_ANNOUNCE_BASE");
    for p in 1..=8u32 {
        assert_eq!(base + p, encode_profile(p as u8) as u32, "profil {p}");
    }
}

#[test]
fn inputs_match_engine_names() {
    let d = defines();
    for n in names() {
        let want = InputId::from_name(n).unwrap().0 as u32;
        assert_eq!(get(&d, &format!("IN_{n}")), want, "IN_{n}");
    }
    assert_eq!(d.keys().filter(|k| k.starts_with("IN_")).count(), names().len());
}

/// Texte des macros ACT/ANN (espaces normalises), pas seulement les constantes.
#[test]
fn macro_text_is_exact() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../firmware/null_pad_codes.h");
    let text = std::fs::read_to_string(&path).unwrap();
    let lines: Vec<String> = text.lines().map(|l| l.split_whitespace().collect::<Vec<_>>().join(" ")).collect();
    for want in [
        "#define ACT(p, i) ZMK_HID_USAGE(HID_USAGE_CONSUMER, (NP_ACTION_BASE + ((p) - 1) * NP_PROFILE_STRIDE + (i)))",
        "#define ANN(p) ZMK_HID_USAGE(HID_USAGE_CONSUMER, (NP_ANNOUNCE_BASE + (p)))",
    ] {
        assert!(lines.iter().any(|l| l == want), "ligne manquante: {want}");
    }
}
