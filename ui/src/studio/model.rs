//! Modèle pur de la keymap du pad : keymap ZMK <-> entrées nullctl, touches natives, garde-fous.
//! Sérialisé tel quel vers l'interface (`{"kind":"action","profile":2,"input":4}`).
use super::client::BehaviorBinding;
use super::hid;
use super::pb::zmk::keymap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const KEY_PRESS: &str = "Key Press";
const CONSUMER_PAGE: u32 = 0x0C;
/// Les usages « action » vont de 0x0C/0x0500 (profil 1, entrée 0) par 32 entrées par profil.
const ACT_BASE: u32 = 0x500;
const ACT_SPAN: u32 = 32;
pub const POSITIONS: usize = 29;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PadBinding {
    Action { profile: u8, input: u8 },
    Native { usage: u32, label: String },
    ProfileMacro { n: u8 },
    Other { behavior: String, p1: u32, p2: u32 },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct PadLayer {
    pub id: u32,
    pub name: String,
    pub bindings: Vec<PadBinding>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct PadModel {
    pub layers: Vec<PadLayer>,
}

#[derive(Serialize, Debug, PartialEq, Eq)]
pub struct PadChange {
    pub layer_id: u32,
    pub pos: Option<u32>,
    pub binding: Option<PadBinding>,
    pub name: Option<String>,
}

fn act_usage(profile: u8, input: u8) -> u32 {
    (CONSUMER_PAGE << 16) | (ACT_BASE + (u32::from(profile) - 1) * ACT_SPAN + u32::from(input))
}

fn native(usage: u32) -> PadBinding {
    let label = hid::name(usage)
        .unwrap_or_else(|| format!("0x{:02X}/0x{:02X}", (usage >> 16) & 0xFF, usage & 0xFFFF));
    PadBinding::Native { usage, label }
}

fn decode_one(b: &keymap::BehaviorBinding, behaviors: &HashMap<u32, String>) -> PadBinding {
    let name = u32::try_from(b.behavior_id)
        .ok()
        .and_then(|id| behaviors.get(&id))
        .cloned();
    match name.as_deref() {
        Some(KEY_PRESS) => {
            // Usage brut : une action ne porte jamais de modificateurs.
            let (page, id) = (b.param1 >> 16, b.param1 & 0xFFFF);
            if page == CONSUMER_PAGE && (ACT_BASE..ACT_BASE + 0x100).contains(&id) {
                let n = id - ACT_BASE;
                PadBinding::Action {
                    profile: (n / ACT_SPAN + 1) as u8,
                    input: (n % ACT_SPAN) as u8,
                }
            } else {
                native(b.param1)
            }
        }
        Some(n) => match n.strip_prefix("np_p").and_then(|d| d.parse::<u8>().ok()) {
            Some(n) => PadBinding::ProfileMacro { n },
            None => PadBinding::Other {
                behavior: n.to_string(),
                p1: b.param1,
                p2: b.param2,
            },
        },
        None => PadBinding::Other {
            behavior: format!("#{}", b.behavior_id),
            p1: b.param1,
            p2: b.param2,
        },
    }
}

pub fn decode(km: &keymap::Keymap, behaviors: &HashMap<u32, String>) -> PadModel {
    PadModel {
        layers: km
            .layers
            .iter()
            .map(|l| PadLayer {
                id: l.id,
                name: l.name.clone(),
                bindings: l
                    .bindings
                    .iter()
                    .map(|b| decode_one(b, behaviors))
                    .collect(),
            })
            .collect(),
    }
}

pub fn encode(b: &PadBinding, behaviors: &HashMap<u32, String>) -> Result<BehaviorBinding, String> {
    let (name, p1, p2) = match b {
        PadBinding::Action { profile, input } => {
            (KEY_PRESS.to_string(), act_usage(*profile, *input), 0)
        }
        PadBinding::Native { usage, .. } => (KEY_PRESS.to_string(), *usage, 0),
        PadBinding::ProfileMacro { n } => (format!("np_p{n}"), 0, 0),
        PadBinding::Other { behavior, p1, p2 } => (behavior.clone(), *p1, *p2),
    };
    let id = behaviors
        .iter()
        .find(|(_, n)| **n == name)
        .map(|(id, _)| *id)
        .ok_or_else(|| format!("Comportement inconnu sur le pad : « {name} »"))?;
    Ok(BehaviorBinding {
        behavior_id: id as i32,
        param1: p1,
        param2: p2,
    })
}

/// Couche par défaut du profil `p` (1..=8), identique à `firmware/null_pad.keymap`.
pub fn default_layer(p: u8) -> Vec<PadBinding> {
    let act = |i: u8| PadBinding::Action {
        profile: p,
        input: i,
    };
    let kp = |id: u32| native((0x07 << 16) | id);
    let mut v: Vec<PadBinding> = if p == 1 {
        let mut v: Vec<_> = [
            0x53, 0x54, 0x55, 0x56, 0x5F, 0x60, 0x61, 0x57, 0x5C, 0x5D, 0x5E, 0, 0x59, 0x5A, 0x5B,
            0x58, 0x62, 0, 0x63, 0,
        ]
        .into_iter()
        .map(kp)
        .collect();
        v[11] = act(11);
        v[17] = PadBinding::Other {
            behavior: "np_00".into(),
            p1: 0,
            p2: 0,
        };
        v[19] = act(19);
        v
    } else {
        (0..20).map(act).collect()
    };
    v.extend((1..=3).map(|n| PadBinding::ProfileMacro { n }));
    v.extend([0xE9, 0xEA, 0xE2].map(|id| native((CONSUMER_PAGE << 16) | id)));
    v.extend([26, 27, 28].map(act));
    v
}

/// Changements de `before` vers `after`, couches appariées par id (une couche ajoutée ou retirée est ignorée :
/// le pad a un nombre de couches fixe).
pub fn diff(before: &PadModel, after: &PadModel) -> Vec<PadChange> {
    let mut out = Vec::new();
    for a in &after.layers {
        let Some(b) = before.layers.iter().find(|l| l.id == a.id) else {
            continue;
        };
        if a.name != b.name {
            out.push(PadChange {
                layer_id: a.id,
                pos: None,
                binding: None,
                name: Some(a.name.clone()),
            });
        }
        for (i, (x, y)) in b.bindings.iter().zip(&a.bindings).enumerate() {
            if x != y {
                out.push(PadChange {
                    layer_id: a.id,
                    pos: Some(i as u32),
                    binding: Some(y.clone()),
                    name: None,
                });
            }
        }
    }
    out
}

/// Garde-fous avant écriture : chaque couche garde un `np_p1` (sinon plus de retour au profil 1) et les noms
/// tiennent dans `max_name_len`. La combo de déverrouillage est une combo ZMK du firmware, absente des
/// bindings : `set_layer_binding` ne peut pas la modifier, il n'y a donc rien à vérifier ici.
pub fn guard(model: &PadModel, max_name_len: usize) -> Result<(), String> {
    for l in &model.layers {
        if l.name.len() > max_name_len {
            return Err(format!(
                "Le nom de la couche « {} » dépasse {max_name_len} caractères.",
                l.name
            ));
        }
        if !l.bindings.contains(&PadBinding::ProfileMacro { n: 1 }) {
            return Err(format!(
                "La couche « {} » doit garder une touche np_p1 (retour au profil 1).",
                l.name
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::studio::fake::{behavior_names, null_pad_keymap};

    fn names() -> HashMap<u32, String> {
        behavior_names()
            .into_iter()
            .map(|(i, n)| (i as u32, n))
            .collect()
    }
    fn model() -> PadModel {
        decode(&null_pad_keymap(), &names())
    }
    fn act(profile: u8, input: u8) -> PadBinding {
        PadBinding::Action { profile, input }
    }

    #[test]
    fn decode_fake_pad() {
        let m = model();
        assert_eq!(m.layers.len(), 8);
        assert_eq!(m.layers[0].name, "DEF");
        assert!(m.layers.iter().all(|l| l.bindings.len() == 29));
        let n7 = &m.layers[0].bindings[4];
        assert_eq!(
            *n7,
            PadBinding::Native {
                usage: 0x0007_005F,
                label: "7".into()
            }
        );
        assert_eq!(m.layers[1].bindings[4], act(2, 4));
        assert_eq!(m.layers[0].bindings[11], act(1, 11));
        assert_eq!(m.layers[7].bindings[28], act(8, 28));
        assert_eq!(m.layers[0].bindings[20], PadBinding::ProfileMacro { n: 1 });
        assert_eq!(
            m.layers[0].bindings[17],
            PadBinding::Other {
                behavior: "np_00".into(),
                p1: 0,
                p2: 0
            }
        );
        assert!(
            matches!(&m.layers[0].bindings[23], PadBinding::Native { label, .. } if label == "Volume +")
        );
    }

    #[test]
    fn roundtrip() {
        let km = null_pad_keymap();
        let m = model();
        for (l, pl) in km.layers.iter().zip(&m.layers) {
            for (b, pb) in l.bindings.iter().zip(&pl.bindings) {
                let e = encode(pb, &names()).unwrap();
                assert_eq!(
                    (e.behavior_id, e.param1, e.param2),
                    (b.behavior_id, b.param1, b.param2)
                );
            }
        }
        let bad = PadBinding::Other {
            behavior: "nope".into(),
            p1: 0,
            p2: 0,
        };
        assert!(encode(&bad, &names()).unwrap_err().contains("nope"));
    }

    #[test]
    fn default_layers_match_firmware() {
        let m = model();
        for p in [1u8, 2, 8] {
            assert_eq!(default_layer(p), m.layers[p as usize - 1].bindings, "p{p}");
        }
    }

    #[test]
    fn diff_is_minimal() {
        let a = model();
        assert!(diff(&a, &a).is_empty());
        let mut b = a.clone();
        b.layers[1].bindings[3] = PadBinding::Native {
            usage: 0x0007_0004,
            label: "A".into(),
        };
        b.layers[2].name = "X".into();
        let d = diff(&a, &b);
        assert_eq!(d.len(), 2);
        assert_eq!(
            d[0],
            PadChange {
                layer_id: 1,
                pos: Some(3),
                binding: Some(b.layers[1].bindings[3].clone()),
                name: None
            }
        );
        assert_eq!(
            d[1],
            PadChange {
                layer_id: 2,
                pos: None,
                binding: None,
                name: Some("X".into())
            }
        );
    }

    #[test]
    fn guard_rules() {
        let mut m = model();
        assert_eq!(guard(&m, 20), Ok(()));
        assert!(guard(&m, 2).unwrap_err().contains("nom"));
        m.layers[3].bindings[20] = act(4, 20);
        assert!(guard(&m, 20).unwrap_err().contains("np_p1"));
    }

    #[test]
    fn json_contract() {
        assert_eq!(
            serde_json::to_string(&act(2, 4)).unwrap(),
            r#"{"kind":"action","profile":2,"input":4}"#
        );
        assert_eq!(
            serde_json::to_string(&PadBinding::ProfileMacro { n: 1 }).unwrap(),
            r#"{"kind":"profile_macro","n":1}"#
        );
    }
}
