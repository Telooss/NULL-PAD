use serde::{Deserialize, Deserializer, Serialize, Serializer};

const ACTION_BASE: u16 = 0x0500;
const PROFILE_BASE: u16 = 0x0600;
const NAMES: [&str; 29] = [
    "K00",
    "K01",
    "K02",
    "K03",
    "K04",
    "K05",
    "K06",
    "K07",
    "K08",
    "K09",
    "K10",
    "K11",
    "K12",
    "K13",
    "K14",
    "K15",
    "K16",
    "K17",
    "K18",
    "K19",
    "M1",
    "M2",
    "M3",
    "ENC_L_CW",
    "ENC_L_CCW",
    "ENC_L_PRESS",
    "ENC_R_CW",
    "ENC_R_CCW",
    "ENC_R_PRESS",
];

/// Entree physique du macropad, 0..=28.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InputId(pub u8);

impl InputId {
    pub fn name(&self) -> &'static str {
        NAMES[self.0 as usize]
    }
    pub fn from_name(s: &str) -> Option<InputId> {
        NAMES.iter().position(|n| *n == s).map(|i| InputId(i as u8))
    }
    pub fn is_rotation(&self) -> bool {
        matches!(self.0, 23 | 24 | 26 | 27)
    }
}

impl Serialize for InputId {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.name())
    }
}

impl<'de> Deserialize<'de> for InputId {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        InputId::from_name(&s)
            .ok_or_else(|| serde::de::Error::custom(format!("entree inconnue: {s}")))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Code {
    Action { profile: u8, input: InputId },
    ProfileAnnounce(u8),
}

pub fn decode(usage: u16) -> Option<Code> {
    match usage {
        0x0500..=0x05FF => {
            let off = usage - ACTION_BASE;
            let (profile, input) = ((off / 32) as u8 + 1, (off % 32) as u8);
            (profile <= 8 && input <= 28).then_some(Code::Action {
                profile,
                input: InputId(input),
            })
        }
        0x0601..=0x0608 => Some(Code::ProfileAnnounce((usage - PROFILE_BASE) as u8)),
        _ => None,
    }
}

pub fn encode_action(profile: u8, input: InputId) -> u16 {
    ACTION_BASE + (profile as u16 - 1) * 32 + input.0 as u16
}

pub fn encode_profile(profile: u8) -> u16 {
    PROFILE_BASE + profile as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_roundtrip() {
        for p in 1..=8 {
            for i in 0..=28 {
                let u = encode_action(p, InputId(i));
                assert_eq!(
                    decode(u),
                    Some(Code::Action {
                        profile: p,
                        input: InputId(i)
                    })
                );
            }
        }
    }
    #[test]
    fn exact_values() {
        assert_eq!(encode_action(1, InputId(0)), 0x0500);
        assert_eq!(encode_action(2, InputId(28)), 0x0500 + 32 + 28);
        assert_eq!(encode_profile(3), 0x0603);
    }
    #[test]
    fn rejects_unknown() {
        assert_eq!(decode(0x00E9), None);
        assert_eq!(decode(0x0500 + 29), None);
        assert_eq!(decode(0x0600), None);
        assert_eq!(decode(0x0609), None);
    }
    #[test]
    fn names() {
        assert_eq!(InputId(5).name(), "K05");
        assert_eq!(InputId::from_name("ENC_R_PRESS"), Some(InputId(28)));
        assert!(InputId(23).is_rotation() && !InputId(25).is_rotation());
    }
    #[test]
    fn name_roundtrip_and_serde_shape() {
        for i in 0..=28 {
            assert_eq!(InputId::from_name(InputId(i).name()), Some(InputId(i)));
        }
        assert_eq!(InputId::from_name("K20_"), None);
        assert_eq!(InputId(21).name(), "M2");
        assert_eq!(
            serde_json::to_string(&InputId(28)).unwrap(),
            "\"ENC_R_PRESS\""
        );
        assert_eq!(
            serde_json::from_str::<InputId>("\"K05\"").unwrap(),
            InputId(5)
        );
        assert!(serde_json::from_str::<InputId>("\"nope\"").is_err());
    }
}
