# Soundboard — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** une touche du NULL/PAD joue un son préchargé à la fois dans le casque et dans le micro virtuel VB-Cable, sans délai de chargement, avec en option le vrai micro mélangé par l'appli.

**Architecture:** module `engine/src/sound/` dans le moteur existant (même processus : pas de saut entre programmes). Les sons sont décodés en mémoire (`Arc<[f32]>`) quand la config change, sur le thread du watcher, jamais à l'appui. Deux sorties rodio (casque, VB-Cable) ont chacune un `Mixer` ; jouer un son = ajouter au mixeur une petite source qui lit le tampon partagé (aucune copie). Le mixeur de rodio démarre les nouvelles sources à la trame suivante : la latence est celle du tampon WASAPI (~10 ms). Un thread audio possède les flux et les rouvre en cas d'erreur. En mode micro « via l'appli », un flux d'entrée cpal remplit un tampon circulaire sans verrou, lu par une source non bloquante branchée sur le mixeur VB-Cable.

**Tech Stack:** Rust stable, crates existantes du moteur + `rodio = { version = "0.23", default-features = false, features = ["playback", "mp3", "vorbis", "wav", "flac"] }` (décodage symphonia, mixeur, conversion de fréquence ; cpal 0.18 réexporté en `rodio::cpal`) + `rtrb = "0.4"` (tampon circulaire temps réel).

**Spec:** `docs/superpowers/specs/2026-11-05-logiciel-macropad-design.md` (§5 action « son », §6 Soundboard).

## Faits vérifiés dans le code des crates (2026-11-05)

- `rodio::DeviceSinkBuilder::from_device(dev)?.with_error_callback(f).open_sink_or_fallback()? -> MixerDeviceSink` ; `.mixer() -> &Mixer` ; `Mixer::add(source)` (Mixer est `Clone + Send + Sync`) ; `.config().sample_rate()/.channel_count()` ; `.log_on_drop(false)` évite un message au drop.
- `rodio::mixer::mixer(channels, rate) -> (Mixer, MixerSource)` crée un mixeur **sans périphérique** : les tests tirent les échantillons de `MixerSource` (aucun son joué).
- `trait Source: Iterator<Item = f32>` : `current_span_len() -> Option<usize>`, `channels() -> ChannelCount` (`NonZero<u16>`), `sample_rate() -> SampleRate` (`NonZero<u32>`), `total_duration() -> Option<Duration>`. Le mixeur convertit fréquence et canaux.
- `rodio::Decoder::try_from(std::fs::File)` décode mp3 / ogg / wav / flac.
- cpal 0.18 : `host.default_output_device()` / `default_input_device()` ouvrent un flux qui **suit tout seul** le périphérique par défaut de Windows (pas de sondage à écrire). Nom : `DeviceTrait::description()?.name()`. `Stream` est `Send` sous WASAPI.
- **Piège** : `rodio::microphone::Microphone` fait `thread::sleep` quand son tampon est vide. Branché sur un mixeur de sortie, il bloquerait le rappel audio. On ne l'utilise pas (Task 5 écrit une source non bloquante).

## Global Constraints

- Windows 11, un seul PC, un seul macropad. Le tout dans la crate `engine` existante.
- Aucun décodage, aucune allocation de tampon audio, aucune ouverture de périphérique sur le chemin d'un appui. Jouer un son = une recherche dans une table + `Mixer::add`.
- Formats : mp3, wav, ogg (vorbis), flac. Son de plus de **5 minutes** refusé (journalisé) : 5 min à 48 kHz stéréo f32 ≈ 115 Mo de mémoire.
- La config existante reste valide : section `sound` avec `#[serde(default)]`, `version` reste 1.
- Une config invalide ne remplace jamais la dernière valide (règle du moteur, inchangée). Un fichier son manquant ou illisible **n'invalide pas** la config : le son est sauté et journalisé.
- Mode micro par défaut : `External` (le logiciel ne touche pas au micro, molette droite = volume micro Windows, comme aujourd'hui). Alan ne veut pas de délai sur sa voix : le mode `App` reste une option, jamais activée sans action de l'utilisateur.
- Aucun test automatique n'ouvre un vrai périphérique audio ni ne joue de son : ceux qui le font sont `#[ignore]`.
- Commandes cargo **sans** `rtk` (non installé sur la machine, décision prise au plan moteur).
- Messages de commit terminés par `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`. Travail sur la branche `feat/soundboard`.

## Modèle de config (source de vérité)

```json
"sound": {
  "master_volume": 100,
  "monitor_device": null,  "monitor_volume": 100,
  "cable_device": null,    "cable_volume": 100,
  "mic_mode": "External",  "mic_device": null, "mic_gain": 100,
  "sounds": [
    { "id": "airhorn", "path": "C:\\sons\\airhorn.mp3", "volume": 80, "on_repeat": "Restart" }
  ]
}
```

- `*_device` : `null` = périphérique par défaut (casque, micro) ou détection auto de VB-Cable (sortie dont le nom contient `CABLE Input`) ; sinon sous-chaîne du nom, sans tenir compte de la casse.
- Volumes et gain : entiers 0..=100. Volume final d'un son = `master × volume/100 × sortie_volume/100`. `monitor_volume: 0` = ne pas s'entendre.
- **Volume de la soundboard** (`master_volume`, demandé par Alan le 2026-11-05) : un seul réglage pour tout ce que la soundboard sort, sons **et**, en mode `App`, le micro qu'elle mélange (« si le micro passe dedans, il est logique de le gérer aussi »). Il agit en direct, y compris sur les sons déjà en cours. Actions `{ "type": "SoundVolume", "delta": 5 }` et `{ "type": "SoundMute" }` (couper / rétablir), assignables à n'importe quelle molette ou touche. Les molettes par défaut ne changent pas (gauche = volume Windows, droite = micro). Comme le gain micro, la valeur réglée vit en mémoire ; celle du fichier s'applique au démarrage et quand elle y change.
- `on_repeat` : `Restart` (coupe et relance) ou `Stop` (rappui pendant la lecture = arrêt).
- Actions nouvelles : `{ "type": "Sound", "id": "airhorn" }`, `{ "type": "StopSounds" }`, `{ "type": "SoundVolume", "delta": 5 }`, `{ "type": "SoundMute" }`.
- `MicVolume` / `MicMute` : en mode `External`, inchangés (micro Windows) ; en mode `App`, ils règlent le gain / la sourdine du micro mélangé par l'appli. Le gain modifié à la molette vit en mémoire (non réécrit dans la config) ; la valeur de la config s'applique au démarrage et chaque fois qu'elle **change** dans le fichier.

## Review Focus

1. **Rappui rapide / spam d'une touche son** → jamais plus d'une voix par son en `Restart`, aucune voix « orpheline » qui joue sans fin ; `Stop` coupe vraiment. Test : Task 4.
2. **VB-Cable absent, casque débranché, périphérique nommé introuvable** → le moteur démarre, les autres actions marchent, les sons jouent sur ce qui reste ; journal clair ; l'icône propose d'installer VB-Cable. Tests : Task 6 (`pick_device`), Task 7.
3. **Fichier son manquant, corrompu, énorme, ou modifié sur disque** → sauté et journalisé, la config reste appliquée ; un fichier modifié (date) est redécodé, un fichier inchangé n'est pas redécodé. Test : Task 3.
4. **Micro « via l'appli » : horloges qui dérivent, rappel d'entrée en retard** → pas de latence qui grossit sans fin, pas de canaux inversés après un saut, silence (pas de blocage) en cas de manque. Test : Task 5.
5. **Arrêt d'un son en pleine forme d'onde** → pas de clic : fondu de 5 ms. Test : Task 2.

---

### Task 1: Config — section `sound` et nouvelles actions

**Files:**
- Modify: `engine/src/config.rs`, `engine/src/dispatch.rs` (bras `match` de `SystemExecutor::run` : `Sound`/`StopSounds`/`SoundVolume`/`SoundMute` → `Err("soundboard non branchee")` provisoire, remplacé en Task 7)
- Create: `engine/tests/fixtures/sound.json`
- Test: `engine/tests/config.rs`

**Interfaces:**
- Produces (dans `config.rs`) :
  - `pub struct SoundSettings { pub master_volume: u8, pub monitor_device: Option<String>, pub monitor_volume: u8, pub cable_device: Option<String>, pub cable_volume: u8, pub mic_mode: MicMode, pub mic_device: Option<String>, pub mic_gain: u8, pub sounds: Vec<SoundDef> }`, `impl Default` (volumes et gain 100, `External`, `None`, liste vide), `#[serde(default)]` au niveau struct.
  - `pub enum MicMode { External, App }`, `pub enum OnRepeat { Restart, Stop }`
  - `pub struct SoundDef { pub id: String, pub path: String, pub volume: u8, pub on_repeat: OnRepeat }` (`volume` défaut 100, `on_repeat` défaut `Restart`)
  - `Config.sound: SoundSettings` avec `#[serde(default)]` ; `Action::Sound { id: String }`, `Action::StopSounds`, `Action::SoundVolume { delta: i8 }`, `Action::SoundMute`.
  - Toutes les structs : `Serialize, Deserialize, Debug, Clone, PartialEq`.

- [ ] **Step 1: Tests qui échouent** dans `tests/config.rs` :
  - `old_config_without_sound_still_loads` : `two_bindings.json` existant → `parse` OK et `cfg.sound == SoundSettings::default()`.
  - `sound_fixture_parses` : `fixtures/sound.json` (un profil, binding `K00` Press → `Sound { id: "airhorn" }`, binding `K01` Press → `StopSounds`, deux sons `airhorn` (volume 80, `Stop`) et `clap` (champs optionnels omis)) → `clap.volume == 100`, `clap.on_repeat == OnRepeat::Restart`, `mic_mode == MicMode::External`.
  - `rejects_bad_sound_settings` (via `invalid()`) : id de son en double ; id vide ; `volume: 101` ; `monitor_volume: 101` ; `master_volume: 101` ; `mic_gain: 101` ; `SoundVolume { delta: 101 }` ; action `Sound` vers un id absent de `sounds` (aussi dans un `Toggle` et dans une `Macro`).
- [ ] **Step 2:** `cargo test -p engine --test config` → les nouveaux tests échouent (compilation).
- [ ] **Step 3: Implémenter.** `validate` vérifie ids uniques et non vides, bornes 0..=100, et que chaque `Sound { id }` (y compris imbriqué) existe : passer l'ensemble des ids à `check_action`.
- [ ] **Step 4:** `cargo test -p engine` → tout passe (les 50 anciens tests compris) ; `cargo clippy -p engine --all-targets -- -D warnings` propre.
- [ ] **Step 5: Commit** `feat(config): section sound et actions Sound / StopSounds`

### Task 2: Voix — source qui lit un tampon partagé

**Files:**
- Create: `engine/src/sound/mod.rs` (`pub mod voice;` puis les modules des tâches suivantes), `engine/src/sound/voice.rs`
- Modify: `engine/src/lib.rs` (`pub mod sound;`), `engine/Cargo.toml` (ajout `rodio`, voir Tech Stack)
- Test: module `tests` de `voice.rs`

**Interfaces:**
- Produces :
  - `pub struct Clip { pub samples: Arc<[f32]>, pub channels: u16, pub rate: u32 }` (échantillons entrelacés)
  - `pub struct VoiceState { stop: AtomicBool, live: AtomicU32 }` avec `pub fn new() -> Arc<Self>`, `pub fn stop(&self)`, `pub fn is_playing(&self) -> bool` (`live > 0 && !stop`)
  - `pub struct Master { level: AtomicU32 /* bits f32 */, muted: AtomicBool }` : `pub fn new(level: f32) -> Arc<Self>`, `pub fn get(&self) -> f32` (0 si coupé), `pub fn set(&self, v: f32)`, `pub fn level(&self) -> f32` (ignore la sourdine), `pub fn toggle_mute(&self) -> bool`.
  - `pub struct Voice` : `pub fn new(clip: &Clip, gain: f32, master: Arc<Master>, state: Arc<VoiceState>) -> Voice` ; chaque échantillon = `sample × gain × master.get()`, `master` relu à chaque début de trame (le volume bouge pendant la lecture) ; `impl Iterator<Item = f32>` et `impl rodio::Source` (`current_span_len` = `None`, `total_duration` = `None`). `new` incrémente `live` ; la voix le décrémente **une seule fois** quand elle se termine (fin du tampon ou fin du fondu) et dans `Drop` si elle n'est pas terminée.
- Fondu : quand `stop` est vu, le gain descend linéairement à 0 en `FADE_FRAMES = rate * 5 / 1000` trames, puis `None`. Le fondu ne commence qu'en début de trame (jamais entre deux canaux d'une même trame).

- [ ] **Step 1: Tests qui échouent** :
  - `plays_all_samples_with_gain` : clip `[0.5; 8]`, 2 canaux, gain 0.5, master 1.0 → 8 échantillons à 0.25 puis `None`.
  - `master_changes_live` : clip `[1.0; 8]`, 2 canaux ; lire 2 échantillons (1.0), `master.set(0.5)` → les suivants valent 0.5 ; `toggle_mute()` → 0.0.
  - `live_counts_voices` : deux `Voice` sur le même état → `live == 2` ; vider l'une → 1 ; `drop` de l'autre non vidée → 0, `is_playing() == false`.
  - `stop_fades_then_ends` : clip de 1 s à 1.0, 48 kHz, 2 canaux ; lire 10 échantillons, `stop()`, puis collecter : la suite est décroissante, compte `2 * 240` échantillons (± une trame), se termine par `None` ; les deux canaux d'une trame ont la même valeur.
- [ ] **Step 2:** `cargo test -p engine sound::voice` → échec.
- [ ] **Step 3: Implémenter.**
- [ ] **Step 4:** tests + clippy OK.
- [ ] **Step 5: Commit** `feat(sound): voix avec fondu de sortie`

### Task 3: Bibliothèque — décodage en mémoire avec cache

**Files:**
- Create: `engine/src/sound/library.rs`
- Test: module `tests` de `library.rs`

**Interfaces:**
- Consumes : `Clip` (Task 2), `SoundDef` / `OnRepeat` (Task 1).
- Produces :
  - `pub struct Loaded { pub clip: Clip, pub volume: f32, pub on_repeat: OnRepeat }` (`volume` = `def.volume / 100`)
  - `pub struct Library { sounds: HashMap<String, Arc<Loaded>>, cache: HashMap<(PathBuf, SystemTime), Clip> }`
  - `pub fn Library::empty() -> Library`, `pub fn get(&self, id: &str) -> Option<Arc<Loaded>>`
  - `pub fn Library::build(defs: &[SoundDef], previous: &Library) -> Library` : pour chaque son, réutilise le `Clip` du cache si `(chemin, date de modification)` est identique, sinon décode. Échec (fichier absent, illisible, > 5 min) → `tracing::warn!` avec l'id et la raison, son absent de la table.
  - `pub fn decode(path: &Path) -> Result<Clip, String>` : `rodio::Decoder::try_from(File)`, canaux et fréquence pris au début du flux ; arrêt et erreur dès que `MAX_SECONDS = 300` est dépassé.
  - `pub fn decoded_count()` **seulement** en `#[cfg(test)]` (compteur statique `AtomicUsize`) pour prouver le cache.

- [ ] **Step 1: Tests qui échouent.** Helper de test `write_wav(path, rate, channels, frames)` : en-tête PCM 16 bits de 44 octets écrit à la main + échantillons. Tests :
  - `decodes_wav` : 0,1 s mono 8 kHz → `clip.channels == 1`, `rate == 8000`, `samples.len() == 800`.
  - `missing_and_corrupt_are_skipped` : un id vers un chemin absent, un id vers un fichier de texte → `build` renvoie une table sans ces deux ids, les sons valides présents.
  - `too_long_is_rejected` : wav mono 8 kHz de 301 s (~4,8 Mo) → absent.
  - `unchanged_file_is_not_decoded_again` : `build` deux fois sur la même liste → `decoded_count` augmente de 1, pas de 2 ; puis réécrire le fichier avec une date différente (`File::set_modified`) → redécodé.
  - `volume_and_repeat_are_copied` : volume 80 → `0.8`, `Stop` conservé.
- [ ] **Step 2:** `cargo test -p engine sound::library` → échec.
- [ ] **Step 3: Implémenter.**
- [ ] **Step 4:** tests + clippy OK.
- [ ] **Step 5: Commit** `feat(sound): bibliotheque de sons decodes en memoire`

### Task 4: Soundboard — jouer, rappui, tout arrêter

**Files:**
- Create: `engine/src/sound/board.rs`
- Test: module `tests` de `board.rs`

**Interfaces:**
- Consumes : `Voice`, `VoiceState`, `Clip` (Task 2), `Library`, `Loaded` (Task 3), `SoundSettings`, `OnRepeat`, `MicMode` (Task 1).
- Produces :
  - `pub struct Outputs { pub monitor: Option<rodio::mixer::Mixer>, pub cable: Option<rodio::mixer::Mixer> }` (`Default` = deux `None`)
  - `pub struct Soundboard { master: Arc<Master>, outputs: ArcSwap<Outputs>, library: ArcSwap<Library>, settings: ArcSwap<SoundSettings>, voices: Mutex<HashMap<String, Arc<VoiceState>>>, mic_gain: AtomicU32 /* bits f32 */, mic_muted: AtomicBool }`
  - `pub fn new() -> Arc<Soundboard>`
  - `pub fn set_outputs(&self, o: Outputs)` (appelé par le thread audio, Task 6)
  - `pub fn apply(&self, s: &SoundSettings)` : reconstruit la bibliothèque (`Library::build` avec l'ancienne en cache), stocke les réglages ; si `mic_gain` (resp. `master_volume`) diffère de l'ancien réglage, remet la valeur en mémoire à celle du fichier.
  - `pub fn change_master(&self, delta: i8) -> f32` (réutilise `clamp_level`), `pub fn toggle_master_mute(&self) -> bool`, `pub fn master(&self) -> Arc<Master>` (pour la source micro, Task 5).
  - `pub fn play(&self, id: &str) -> Result<(), String>` : son inconnu ou non chargé → `Err` ; selon `on_repeat` et la voix en cours (`is_playing`) : `Restart` → `stop()` l'ancienne puis joue ; `Stop` → `stop()` et ne rejoue pas. Jouer = une `Voice` par sortie présente, gain `loaded.volume × sortie_volume/100`, sortie à volume 0 sautée, un seul `VoiceState` partagé enregistré sous l'id.
  - `pub fn stop_all(&self)` : `stop()` sur toutes les voix, table vidée.
  - `pub fn mic_via_app(&self) -> bool`, `pub fn change_mic_gain(&self, delta: i8) -> f32` (réutilise `actions::audio::clamp_level`), `pub fn toggle_mic_mute(&self) -> bool`, `pub fn mic_gain(&self) -> f32` (0 si sourdine) — lus par la source micro (Task 5) via `Arc<Soundboard>`.

- [ ] **Step 1: Tests qui échouent.** Les mixeurs viennent de `rodio::mixer::mixer(2, 48000)` ; le test tire les échantillons du `MixerSource`. La bibliothèque de test est construite à partir de wav générés (helper de Task 3, rendu `pub(crate)` en `#[cfg(test)]`). Tests :
  - `play_reaches_both_outputs_with_volumes` : son 80 %, `monitor_volume 50`, `cable_volume 100` → premier échantillon non nul ≈ `0.4 × v` sur monitor et `0.8 × v` sur cable.
  - `zero_monitor_volume_skips_monitor` : `monitor_volume 0` → monitor reste à 0.0, cable joue.
  - `restart_keeps_one_voice` : `play` ×3 en `Restart` → une seule voix vivante après les fondus (compter via `VoiceState` : les anciennes ont `stop`).
  - `stop_mode_toggles` : `Stop` : `play` → joue ; `play` → arrête, et un 3ᵉ `play` rejoue.
  - `stop_all_silences` : deux sons, `stop_all`, tirer 1 000 échantillons → les derniers sont 0.0.
  - `unknown_sound_is_err` et `no_outputs_is_ok` (aucune sortie : `Ok`, rien ne casse).
  - `mic_gain_kept_across_apply` : `apply(gain 100)`, `change_mic_gain(-20)` → 0.8 ; `apply` du même réglage → toujours 0.8 ; `apply(gain 50)` → 0.5. Même test pour `master_volume` / `change_master`.
  - `master_scales_playing_sound` : un son en cours ; `change_master(-50)` → les échantillons suivants des deux sorties sont divisés par deux ; `toggle_master_mute()` → 0.0 ; encore → reprise.
- [ ] **Step 2:** `cargo test -p engine sound::board` → échec.
- [ ] **Step 3: Implémenter.**
- [ ] **Step 4:** tests + clippy OK.
- [ ] **Step 5: Commit** `feat(sound): soundboard (lecture, rappui, arret)`

### Task 5: Micro « via l'appli » — source non bloquante

**Files:**
- Create: `engine/src/sound/mic.rs`
- Modify: `engine/Cargo.toml` (`rtrb = "0.4"`)
- Test: module `tests` de `mic.rs`

**Interfaces:**
- Consumes : `Soundboard::mic_gain()` (Task 4).
- Produces :
  - `pub fn mic_channel(channels: u16, rate: u32, board: Arc<Soundboard>) -> (rtrb::Producer<f32>, MicSource)` : capacité 200 ms.
  - `pub struct MicSource` : `impl Iterator<Item = f32> + rodio::Source` (`channels`/`rate` du micro, durée `None`, ne renvoie **jamais** `None`).
- Algorithme de `next()` (constantes `TARGET_MS = 15`, `MAX_MS = 50`), décisions prises **en début de trame** seulement :
  1. Pas amorcé : si le tampon contient moins de `TARGET_MS` → renvoyer 0.0 ; sinon amorcé.
  2. Amorcé et tampon vide → désamorcer, renvoyer 0.0.
  3. Tampon au-delà de `MAX_MS` → jeter des trames entières jusqu'à revenir à `TARGET_MS`.
  4. Sinon `pop() × board.mic_gain() × master.get()` (le volume de la soundboard s'applique aussi au micro qu'elle mélange).

- [ ] **Step 1: Tests qui échouent** (48 kHz, 2 canaux ; le producteur est poussé à la main) :
  - `silence_until_primed` : pousser 10 ms → la source rend des 0.0 ; pousser 10 ms de plus → elle rend les valeurs poussées.
  - `underrun_gives_silence_and_reprimes` : après avoir tout lu → 0.0, puis 0.0 tant qu'on n'a pas repoussé 15 ms.
  - `drift_is_trimmed_frame_aligned` : pousser 100 ms où le canal gauche vaut +1 et le droit −1 → après lecture de deux échantillons, il reste ≈ 15 ms dans le tampon, et tout ce qui est lu alterne +1, −1 (jamais deux du même signe à la suite).
  - `gain_and_mute_apply` : `change_mic_gain(-50)` → valeurs × 0.5 ; `toggle_mic_mute()` → 0.0 ; sourdine levée puis `change_master(-50)` → valeurs × 0.25 (gain micro 0.5 × master 0.5).
  - `never_returns_none` : source sans producteur actif (producteur lâché) → toujours `Some(0.0)`.
- [ ] **Step 2:** `cargo test -p engine sound::mic` → échec.
- [ ] **Step 3: Implémenter.**
- [ ] **Step 4:** tests + clippy OK.
- [ ] **Step 5: Commit** `feat(sound): pont micro non bloquant`

### Task 6: Périphériques — thread audio, réouverture, VB-Cable

**Files:**
- Create: `engine/src/sound/devices.rs`
- Test: module `tests` de `devices.rs` (logique pure) + un test `#[ignore]` qui ouvre les vrais périphériques

**Interfaces:**
- Consumes : `Soundboard::set_outputs`, `Outputs` (Task 4), `mic_channel` (Task 5), `SoundSettings`, `MicMode` (Task 1).
- Produces :
  - `pub fn pick_device(names: &[String], wanted: Option<&str>, auto: Option<&str>) -> Option<usize>` : `wanted` = sous-chaîne sans casse ; si `wanted` est `None`, `auto` (sous-chaîne) ; si les deux sont `None` → `None` (= périphérique par défaut).
  - `pub const CABLE_HINT: &str = "CABLE Input";`
  - `pub struct AudioHandle { tx: Sender<AudioCmd> }`, `pub fn apply(&self, s: &SoundSettings)`.
  - `pub fn start_audio(board: Arc<Soundboard>) -> AudioHandle` : lance le thread audio. Il possède les `MixerDeviceSink` et le flux micro, et attend des commandes `Apply(SoundSettings)` ou `StreamError` (`recv_timeout(5 s)`).
  - `pub fn cable_present() -> bool` (liste les sorties, cherche `CABLE_HINT`).
- Comportement du thread :
  - **Rien à faire, rien d'ouvert** : si `sounds` est vide et `mic_mode` vaut `External`, le thread ferme tout et ne garde aucun périphérique (ni ressource audio, ni flux ouvert pendant les tests `e2e`).
  - **Ouverture** (à `Apply` si les réglages de périphériques ont changé, ou après une erreur) : casque = `default_output_device()` si `monitor_device` est `None` (cpal suit alors le défaut de Windows), sinon `pick_device` sur les noms des sorties ; VB-Cable = `pick_device(noms, cable_device, Some(CABLE_HINT))`. Introuvable → `warn!` et sortie `None`. Rappel d'erreur → envoie `StreamError` (pour `DeviceNotAvailable` et `StreamInvalidated`). `log_on_drop(false)` sur chaque sink.
  - **Mode `App`** : ouvrir le micro (`default_input_device()` ou `pick_device` sur les entrées), `default_input_config()` ; format autre que `F32` → `error!` et pas de micro (`shortcut:` commentaire : WASAPI partagé donne du f32, élargir si un cas réel apparaît). `build_input_stream` pousse dans le `Producer` (ce qui ne rentre pas est jeté) ; la `MicSource` est ajoutée au mixeur VB-Cable. Sans VB-Cable, pas de micro (journalisé).
  - **Après une erreur ou un échec d'ouverture** : `set_outputs(Outputs::default())`, puis nouvelle tentative à chaque timeout de 5 s tant qu'il manque une sortie demandée.
  - Une fois ouvert : `set_outputs(...)` et `info!` avec le nom, la fréquence et la taille du tampon de chaque sortie.

- [ ] **Step 1: Tests qui échouent** : `pick_device` — choix par sous-chaîne sans casse (`"focusrite"` trouve `"Speakers (Focusrite USB Audio)"`) ; `wanted` prioritaire sur `auto` ; `auto` trouve `"CABLE Input (VB-Audio Virtual Cable)"` ; rien trouvé → `None` ; deux correspondances → la première. Test `#[ignore] opens_real_devices` : `start_audio` + `apply(SoundSettings::default())`, attendre 2 s, vérifier qu'au moins `monitor` est `Some` (ne joue aucun son).
- [ ] **Step 2:** `cargo test -p engine sound::devices` → échec.
- [ ] **Step 3: Implémenter.**
- [ ] **Step 4:** tests + clippy OK. Ne **pas** lancer le test `#[ignore]` sans l'accord d'Alan.
- [ ] **Step 5: Commit** `feat(sound): thread audio, peripheriques et reouverture`

### Task 7: Branchement dans le moteur, icône, documentation

**Files:**
- Modify: `engine/src/dispatch.rs` (`SystemExecutor`), `engine/src/watch.rs`, `engine/src/main.rs`, `engine/src/tray.rs`, `engine/README.md`, `README.md` (tableau d'état : soundboard faite, à valider)
- Test: `engine/src/dispatch.rs` (module `tests`), `engine/tests/watch.rs`, `engine/tests/e2e.rs` inchangé et vert

**Interfaces:**
- Consumes : `Soundboard` (Task 4), `start_audio`, `AudioHandle`, `cable_present` (Task 6).
- Produces / changes :
  - `pub struct SystemExecutor { pub sound: Arc<Soundboard> }`. `Sound { id }` → `sound.play(&id)` ; `StopSounds` → `sound.stop_all()` ; `SoundVolume { delta }` → `sound.change_master(delta)` ; `SoundMute` → `sound.toggle_master_mute()` ; `MicVolume` / `MicMute` → si `sound.mic_via_app()`, gain / sourdine de l'appli, sinon micro Windows comme aujourd'hui.
  - `watch_config(path, target, on_reload: impl Fn(&Config) + Send + 'static)` : appelé sur le thread du watcher après chaque config **acceptée** (jamais pour une rejetée).
  - `main` : `let board = Soundboard::new(); let audio = start_audio(board.clone());` puis, avec la config chargée, `board.apply(&cfg.sound); audio.apply(&cfg.sound);` ; même chose dans `on_reload`. `SystemExecutor { sound: board }`.
  - `create_tray(cable_missing: bool)` : si `true`, entrée « VB-Cable absent : l'installer… » qui ouvre `https://vb-audio.com/Cable/` avec `launch::open`. `shortcut:` état lu au démarrage seulement, l'interface (plan 6) affichera l'état en direct.

- [ ] **Step 1: Tests qui échouent** :
  - `tests/watch.rs` : `on_reload_called_only_for_valid` : un compteur incrémenté par `on_reload` ; écriture valide → +1 ; écriture tronquée → inchangé.
  - Les tests de `dispatch.rs` utilisent toujours leur `Rec` : vérifier que `Sound` et `StopSounds` passent par le worker (`Rec` les voit, sans thread de plus).
- [ ] **Step 2:** `cargo test -p engine` → échec (signature de `watch_config`).
- [ ] **Step 3: Implémenter.** Adapter les appels existants à `watch_config` (tests compris).
- [ ] **Step 4: Vérifier** : `cargo test -p engine` (tout vert, `e2e` compris), `cargo clippy -p engine --all-targets -- -D warnings`, `cargo build --locked --release`.
- [ ] **Step 5: Documentation.** `engine/README.md` : section « Soundboard » (modèle de config ci-dessus, VB-Cable : installer, puis choisir « CABLE Output » comme micro dans le jeu / Discord / OBS ; mode `External` : brancher le vrai micro sur VB-Cable par Windows « Écouter ce périphérique » ou par l'interface audio ; mode `App` : latence ~25–40 ms, micro muet si le moteur est fermé, et ne pas activer en plus « Écouter ce périphérique » sinon le micro est doublé). Section « Molettes » : exemple de config avec `SoundVolume` sur une molette. Ajouter à la liste « À tester à la main » : son dans le casque ; `SoundVolume` sur une molette baisse un son en cours (casque et VB-Cable) et, en mode `App`, la voix aussi ; son dans « CABLE Output » (enregistreur Windows ou Discord) ; rappui `Restart` et `Stop` ; `StopSounds` ; débrancher le casque pendant un son puis rebrancher ; changer la sortie par défaut de Windows ; VB-Cable absent → entrée dans l'icône ; mode `App` : voix audible dans « CABLE Output », molette droite = gain, clic = sourdine, latence ressentie.
- [ ] **Step 6: Commit** `feat(engine): soundboard branchee (actions, rechargement, icone)`
