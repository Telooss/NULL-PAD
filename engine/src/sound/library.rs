//! Bibliotheque : sons decodes une fois en memoire, avec cache par (chemin, date).

use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use crate::config::{OnRepeat, SoundDef};
use rodio::Source;

use crate::sound::voice::Clip;

const MAX_SECONDS: usize = 300;

pub struct Loaded {
    pub clip: Clip,
    pub volume: f32,
    pub on_repeat: OnRepeat,
}

pub struct Library {
    sounds: HashMap<String, Arc<Loaded>>,
    cache: HashMap<(PathBuf, SystemTime), Clip>,
}

#[cfg(test)]
static DECODED: std::sync::Mutex<Vec<PathBuf>> = std::sync::Mutex::new(Vec::new());

/// Nombre de decodages du fichier `path` (les tests tournent en parallele).
#[cfg(test)]
pub fn decoded_count(path: &Path) -> usize {
    DECODED.lock().unwrap().iter().filter(|p| *p == path).count()
}

impl Library {
    pub fn empty() -> Library {
        Library { sounds: HashMap::new(), cache: HashMap::new() }
    }

    pub fn get(&self, id: &str) -> Option<Arc<Loaded>> {
        self.sounds.get(id).cloned()
    }

    pub fn build(defs: &[SoundDef], previous: &Library) -> Library {
        let mut lib = Library::empty();
        for def in defs {
            let path = PathBuf::from(&def.path);
            let modified = match std::fs::metadata(&path).and_then(|m| m.modified()) {
                Ok(t) => t,
                Err(e) => {
                    tracing::warn!("son '{}' ignore : {}", def.id, e);
                    continue;
                }
            };
            let key = (path, modified);
            let clip = match lib.cache.get(&key).or_else(|| previous.cache.get(&key)) {
                Some(c) => c.clone(),
                None => match decode(&key.0) {
                    Ok(c) => c,
                    Err(e) => {
                        tracing::warn!("son '{}' ignore : {}", def.id, e);
                        continue;
                    }
                },
            };
            lib.cache.insert(key, clip.clone());
            lib.sounds.insert(
                def.id.clone(),
                Arc::new(Loaded { clip, volume: f32::from(def.volume) / 100.0, on_repeat: def.on_repeat }),
            );
        }
        lib
    }
}

pub fn decode(path: &Path) -> Result<Clip, String> {
    #[cfg(test)]
    DECODED.lock().unwrap().push(path.to_path_buf());
    let file = File::open(path).map_err(|e| e.to_string())?;
    let dec = rodio::Decoder::try_from(file).map_err(|e| e.to_string())?;
    let channels = dec.channels().get();
    let rate = dec.sample_rate().get();
    let max = MAX_SECONDS * rate as usize * channels as usize;
    let mut samples = Vec::new();
    for s in dec {
        if samples.len() >= max {
            return Err(format!("plus de {} min", MAX_SECONDS / 60));
        }
        samples.push(s);
    }
    if samples.is_empty() {
        return Err("fichier vide".into());
    }
    Ok(Clip { samples: samples.into(), channels, rate })
}

/// Ecrit un wav PCM 16 bits (rampe deterministe non nulle).
#[cfg(test)]
pub(crate) fn write_wav(path: &Path, rate: u32, channels: u16, frames: usize) {
    let n = frames * channels as usize;
    let data_len = (n * 2) as u32;
    let mut b = Vec::with_capacity(44 + n * 2);
    b.extend(b"RIFF");
    b.extend((36 + data_len).to_le_bytes());
    b.extend(b"WAVEfmt ");
    b.extend(16u32.to_le_bytes());
    b.extend(1u16.to_le_bytes());
    b.extend(channels.to_le_bytes());
    b.extend(rate.to_le_bytes());
    b.extend((rate * channels as u32 * 2).to_le_bytes());
    b.extend((channels * 2).to_le_bytes());
    b.extend(16u16.to_le_bytes());
    b.extend(b"data");
    b.extend(data_len.to_le_bytes());
    for i in 0..n {
        b.extend((1000 + (i % 100) as i16).to_le_bytes());
    }
    std::fs::write(path, b).unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn tmp(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("nullpad-lib-{}-{}.wav", std::process::id(), name))
    }

    fn def(id: &str, path: &Path) -> SoundDef {
        SoundDef { id: id.into(), path: path.to_string_lossy().into(), volume: 100, on_repeat: OnRepeat::Restart }
    }

    #[test]
    fn decodes_wav() {
        let p = tmp("decodes");
        write_wav(&p, 8000, 1, 800);
        let clip = decode(&p).unwrap();
        std::fs::remove_file(&p).ok();
        assert_eq!((clip.channels, clip.rate, clip.samples.len()), (1, 8000, 800));
    }

    #[test]
    fn missing_and_corrupt_are_skipped() {
        let (ok, bad) = (tmp("ok"), tmp("corrupt"));
        write_wav(&ok, 8000, 1, 80);
        std::fs::write(&bad, "pas de l'audio").unwrap();
        let defs = [def("ok", &ok), def("bad", &bad), def("gone", &tmp("absent"))];
        let lib = Library::build(&defs, &Library::empty());
        std::fs::remove_file(&ok).ok();
        std::fs::remove_file(&bad).ok();
        assert!(lib.get("ok").is_some());
        assert!(lib.get("bad").is_none() && lib.get("gone").is_none());
    }

    #[test]
    fn too_long_is_rejected() {
        let p = tmp("long");
        write_wav(&p, 8000, 1, 301 * 8000);
        let lib = Library::build(&[def("long", &p)], &Library::empty());
        std::fs::remove_file(&p).ok();
        assert!(lib.get("long").is_none());
    }

    #[test]
    fn unchanged_file_is_not_decoded_again() {
        let p = tmp("cache");
        write_wav(&p, 8000, 1, 80);
        let defs = [def("a", &p), def("b", &p)];
        let l1 = Library::build(&defs, &Library::empty());
        assert_eq!(decoded_count(&p), 1, "un seul decodage pour deux ids");
        let l2 = Library::build(&defs, &l1);
        assert_eq!(decoded_count(&p), 1, "cache reutilise");
        let f = File::options().write(true).open(&p).unwrap();
        f.set_modified(SystemTime::now() + Duration::from_secs(60)).unwrap();
        drop(f);
        let l3 = Library::build(&defs, &l2);
        std::fs::remove_file(&p).ok();
        assert_eq!(decoded_count(&p), 2, "date changee : redecode");
        assert!(l3.get("a").is_some());
    }

    #[test]
    fn volume_and_repeat_are_copied() {
        let p = tmp("vol");
        write_wav(&p, 8000, 1, 80);
        let d = SoundDef { volume: 80, on_repeat: OnRepeat::Stop, ..def("v", &p) };
        let lib = Library::build(&[d], &Library::empty());
        std::fs::remove_file(&p).ok();
        let l = lib.get("v").unwrap();
        assert_eq!((l.volume, l.on_repeat), (0.8, OnRepeat::Stop));
    }
}
