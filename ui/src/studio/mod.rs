//! Client du protocole ZMK Studio (RPC protobuf sur port série).
pub mod client;
#[cfg(test)]
pub(crate) mod fake;
pub mod framing;
pub mod port;

#[allow(clippy::all, dead_code)]
pub mod pb {
    pub mod zmk {
        pub mod studio {
            include!(concat!(env!("OUT_DIR"), "/zmk.studio.rs"));
        }
        pub mod core {
            include!(concat!(env!("OUT_DIR"), "/zmk.core.rs"));
        }
        pub mod keymap {
            include!(concat!(env!("OUT_DIR"), "/zmk.keymap.rs"));
        }
        pub mod behaviors {
            include!(concat!(env!("OUT_DIR"), "/zmk.behaviors.rs"));
        }
        pub mod meta {
            include!(concat!(env!("OUT_DIR"), "/zmk.meta.rs"));
        }
    }
}
