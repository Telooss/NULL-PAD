# Interface nullctl — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** l'appli de réglage NULL/CONTROL (`nullctl.exe`). Elle montre le NULL/PAD en 3D avec la fonction de chaque touche, suit les appuis en direct et permet de tout régler sans toucher au JSON, grâce à un mode configuration à brouillon par profil.

**Architecture :**
- **Moteur** : renommé `nullctl-engine.exe`. Il gagne deux named pipes :
  - `\\.\pipe\nullctl-events` (sortant) : il diffuse ses événements ;
  - `\\.\pipe\nullctl-control` (requête/réponse) : il répond à `status` et aux boutons « Tester ».
- **Interface** : une crate Tauri 2, `ui/`, membre du workspace. Le côté Rust lit la config, la valide avec `engine::config::parse` et l'écrit de façon atomique. Il relaie aussi les deux pipes en événements et commandes Tauri.
- **Partie web** : HTML/CSS/JS en modules ES, sans bundler ni framework. Elle reprend tel quel le design system « nullctl » copié dans `ui/design/` (tokens, `bundle.css`, `bundle.js` avec `NullCtl.pad3d`, modèle `nullpad.json`).
- **Tests** :
  - la logique pure (légendes, brouillon, dépôt de fichier, catalogue d'actions) vit dans des modules testés par `node --test` ;
  - un `bridge.js` simule le côté Rust quand `window.__TAURI__` est absent : la partie web s'ouvre alors dans un navigateur ordinaire pour les captures d'écran.

**Tech Stack :**
- Rust stable, `tauri = "2.12"`, `tauri-build = "2.7"`, `tauri-plugin-dialog = "2.8"`, `tauri-plugin-single-instance = "2.5"` (dernières versions stables, vérifiées sur crates.io le 2026-11-07 ; ne pas prendre les `3.0.0-alpha`).
- `tauri-cli` 2.12 pour le build (`cargo install tauri-cli --version ^2.12 --locked`).
- `windows` 0.62 (déjà là) pour les pipes.
- three.js **0.160.0** (version des maquettes) et lucide **0.344.0**, copiés dans `ui/web/vendor/` : l'appli fonctionne hors ligne, sans CDN.
- Node 24 (présent) uniquement pour `node --test`.

**Spec :**
- `docs/superpowers/specs/2026-11-07-interface-design.md` (§1–12, qui gagnent pour l'interface) ;
- `docs/superpowers/specs/2026-11-05-logiciel-macropad-design.md` (§7, clarté).

**Design system :** https://claude.ai/artifact/JpPjDZvwtqksmzRf54H5Cw, à copier dans `ui/design/` (Task 4).

## Global Constraints

- **Noms** :
  - pad = **NULL/PAD**, logiciel = **NULL/CONTROL (nullctl)** ;
  - exécutables `nullctl-engine.exe` (moteur) et `nullctl.exe` (interface), dans le même dossier ;
  - données dans `%APPDATA%\nullctl` (migrées depuis `%APPDATA%\NULL-PAD`) ;
  - mutex `Local\nullctl-engine` ;
  - infobulle « NULL/CONTROL ».
- **Direction artistique** : le design system s'applique **tel quel** (tokens, classes `nc-*`, polices, décor seulement dans la scène, glitch de 0,36 s seulement sur un changement de profil, une sauvegarde ou une création). Aucun nouvel effet. Test final : tous les effets coupés (classe `no-fx` sur `.nc-app`), l'appli reste claire.
- **3D** :
  - rendu **à la demande** uniquement : aucune boucle `requestAnimationFrame` permanente, et rien n'est dessiné quand la fenêtre est cachée ;
  - densité de pixels `min(devicePixelRatio, 1.5)` ;
  - les textures de légende ne sont redessinées que quand leur contenu change.
- **Le pad ne change qu'à l'enregistrement** :
  - en mode configuration, rien n'est écrit sur le disque avant « Enregistrer » ;
  - l'écriture se fait d'un coup : fichier temporaire, puis `rename` sur `config.json`.
- **Validation** : avant toute écriture, l'interface valide la config entière avec `engine::config::parse`. Aucune règle de validation n'est réécrite en JS.
- **Indépendance du moteur** : il ne bloque jamais à cause de l'interface. Un client de pipe lent perd ses messages, et sans client rien n'est écrit. Le moteur marche pareil sans interface.
- **Config** : les configs existantes restent valides. `label` est un champ optionnel de `Binding`, et `version` reste 1.
- **Touches natives** : la couche 0 du firmware hors actions (`K11`, `K19` exceptées). Elles n'acceptent aucune liaison du moteur, et l'interface le refuse.
- **Tests** :
  - aucun test automatique ne lance l'interface Tauri, ne touche au vrai `%APPDATA%` et n'injecte de frappes ;
  - les pipes sont testés avec des noms uniques par test ;
  - `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings` et `node --test ui/web/test/` doivent passer ;
  - les commandes cargo se lancent **sans** `rtk`.
- **Git** : messages de commit terminés par `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`, sur la branche `feat/interface`.

## Review Focus

1. **Enregistrer pendant que le moteur lit, ou moteur arrêté** : jamais de `config.json` à moitié écrit. Le brouillon est gardé tant que le moteur n'a pas confirmé. Tests : Task 5 (écriture atomique) et Task 9 (machine à états de la sauvegarde).
2. **Interface fermée, plantée ou lente pendant que le pad est utilisé** : aucun effet sur la latence ni sur les actions. Test : Task 2 (client qui ne lit pas, puis 10 000 événements).
3. **Changement de profil sur le pad, ou fermeture, avec un brouillon modifié** : rien n'est perdu sans confirmation, et l'affichage ne saute pas. Test : Task 6 (`draft.js`) et Task 9.
4. **Dépôt d'un fichier sur une touche native, sur M1–M3, sur une molette, ou dépôt de plusieurs fichiers** : un comportement net et un message. Test : Task 6 (`drop.js`).
5. **GPU au repos** : zéro image dessinée quand rien ne change. Test : Task 7 (compteur d'images exposé en mode test, qui reste stable sur 2 s d'inactivité).

---

### Task 1: Renommage nullctl et champ `label`

**Files:** Modify `engine/Cargo.toml` (`[[bin]] name = "nullctl-engine"`), `engine/src/paths.rs`, `engine/src/main.rs`, `engine/src/tray.rs`, `engine/src/autostart.rs`, `engine/src/config.rs`, `engine/tests/{e2e,simulate}.rs` (`CARGO_BIN_EXE_nullctl-engine`), `engine/README.md`, `README.md`, spec §10.

**Interfaces — Produces :**
- `pub const APP_DIR: &str = "nullctl"` et `pub const OLD_APP_DIR: &str = "NULL-PAD"`.
- `pub fn migrate_app_dir(base: &Path) -> io::Result<bool>` dans `paths.rs` : si `base/NULL-PAD` existe et que `base/nullctl` n'existe pas, renomme le premier et renvoie `true`. Elle est appelée au début de `main` avec `dirs::config_dir()`.
- Démarrage avec Windows :
  - la valeur `Run` s'appelle `nullctl` ;
  - au lancement, une ancienne valeur `NULL-PAD` présente est supprimée, et la nouvelle est créée seulement si l'ancienne existait. Le choix de l'utilisateur est donc gardé.
- Nouvelle entrée du menu de l'icône, « Ouvrir nullctl » : elle lance `nullctl.exe`, pris dans le dossier de `current_exe()`. Si le fichier est absent → `warn!`.
- `Binding.label: Option<String>`, en `#[serde(default, skip_serializing_if = "Option::is_none")]`. Le moteur l'ignore.

- [ ] **Step 1: Tests qui échouent.**
  - `migrate_app_dir` sur un dossier temporaire : renomme l'ancien dossier quand il est seul ; ne fait rien si le nouveau existe déjà ; ne fait rien si aucun n'existe.
  - `config.rs` : une liaison avec `"label": "HORN"` se relit à l'identique ; sans `label`, la valeur est `None` et le champ n'est pas écrit.
- [ ] **Step 2:** RED. **Step 3:** implémenter. Les chaînes visibles disent « NULL/CONTROL » ou « nullctl » ; le matériel reste « NULL/PAD ».
- [ ] **Step 4:** `cargo test -p engine` et clippy → vert. `cargo build --release` doit produire `target/release/nullctl-engine.exe`.
- [ ] **Step 5: Commit** `refactor(engine): nom nullctl, migration du dossier, champ label`

### Task 2: Pipe d'événements du moteur

**Files:** Create `engine/src/ipc.rs` (+ `pub mod ipc;`). Modify `engine/src/dispatch.rs`, `engine/src/input_win.rs`, `engine/src/main.rs`, `engine/src/watch.rs` (`on_reject`). Test `engine/tests/ipc.rs`.

**Interfaces — Produces :**
- `pub enum Event`, avec `Serialize` et `#[serde(tag = "type", rename_all = "snake_case")]` :
  - `Key { input: String, down: bool, profile: u8 }`
  - `Action { input: String, trigger: Trigger, kind: String, ok: bool, error: Option<String> }`
  - `Profile { profile: u8 }`
  - `Link { link: Link }`, avec `Link { Usb, Ble, None }`
  - `Config { ok: bool, error: Option<String> }`
  - `kind` = le nom de la variante d'`Action` (« Sound », « ObsScene »…).
- `#[derive(Clone)] pub struct Events` :
  - `pub fn emit(&self, e: &Event)` : sérialise en une ligne et l'envoie avec `try_send` à chaque client ; un client plein perd la ligne ; sans client, ne fait rien.
  - `pub fn serve_events(name: &str) -> io::Result<Events>` : un thread accepte les clients en boucle (`CreateNamedPipeW` en `PIPE_ACCESS_OUTBOUND`, `PIPE_UNLIMITED_INSTANCES`, puis `ConnectNamedPipe`). Chaque client a un `sync_channel(256)` et un thread d'écriture. Une erreur d'écriture retire le client.
  - Nom de production : `\\.\pipe\nullctl-events`.
- **Où le moteur émet** :
  - `Dispatcher` émet `Key` sur chaque front et `Profile` sur une annonce.
  - `SystemExecutor::run` renvoie maintenant `Result<(), String>`, et le worker émet `Action` avec `ok` ou `error`. Les actions lancées en thread (Launch, Command, Macro) émettent depuis leur thread. Le `Dispatcher` transmet l'entrée et le déclencheur.
  - `input_win` émet `Link` à l'arrivée et au retrait du pad. Le chemin de périphérique Raw Input contient `BTHLE` ou `{00001812-` → `Ble`, sinon `Usb`.
  - `watch_config` émet `Config` : `ok: true` après `on_reload`, et `ok: false` avec la raison en cas de rejet.
- `Dispatcher::new` et `SystemExecutor` reçoivent un `Events`. Les tests utilisent `Events::none()`, qui n'émet rien.

- [ ] **Step 1: Tests qui échouent** (`engine/tests/ipc.rs`, un nom de pipe unique par test, client ouvert avec `std::fs::File::open` sur le chemin du pipe) :
  - `client_receives_events_as_json_lines` : `emit(Profile{2})` donne la ligne `{"type":"profile","profile":2}`.
  - `two_clients_both_receive`.
  - `slow_client_never_blocks` : un client connecté qui ne lit jamais ; 10 000 `emit` prennent moins de 200 ms.
  - `emit_without_client_is_noop`.
  - Plus des tests unitaires : sérialisation exacte de chaque variante d'`Event`, et `Dispatcher` qui émet `Key` et `Profile`, avec un `Events` de test qui enregistre au lieu d'écrire.
- [ ] **Step 2:** RED. **Step 3:** implémenter. **Step 4:** `cargo test -p engine` (e2e compris) et clippy → vert.
- [ ] **Step 5: Commit** `feat(engine): pipe d'evenements pour l'interface`

### Task 3: Pipe de contrôle du moteur

**Files:** Modify `engine/src/ipc.rs`, `engine/src/main.rs`, `engine/src/obs.rs` et `engine/src/midi.rs` (fonctions de test). Test `engine/tests/ipc.rs`.

**Interfaces — Produces :**
- `pub fn serve_control(name: &str, handler: impl Fn(Request) -> Response + Send + Sync + 'static) -> io::Result<()>` : un pipe duplex, une requête JSON par ligne, une réponse par ligne. Plusieurs clients à la fois. Une requête illisible reçoit `{"ok":false,"error":"requete invalide"}`.
- `Request` (avec `#[serde(tag = "cmd", rename_all = "snake_case")]`) : `Status`, `TestObs`, `TestMidi`.
- `Response { ok: bool, error: Option<String>, data: Option<serde_json::Value> }`.
- Réponses :
  - `Status` donne `data = {"profile": u8|null, "link": "usb"|"ble"|"none", "version": "<CARGO_PKG_VERSION>"}`. Le moteur garde le dernier profil et le dernier lien dans un `ArcSwap`.
  - `TestObs` : `obs::test_connection(&ObsSettings) -> Result<(), String>`, une connexion avec Identify, sans requête, en 2 s au plus. Elle s'exécute sur le thread du client du pipe, jamais sur le worker.
  - `TestMidi` : `midi.send(&[cc(1, 127, 0)])`. C'est un CC 127 à valeur 0, inoffensif.
- Nom de production : `\\.\pipe\nullctl-control`.

- [ ] **Step 1: Tests qui échouent** :
  - `status_roundtrip` : avec un handler factice ;
  - `bad_request_gets_error` ;
  - `two_clients_in_parallel` ;
  - `test_connection` contre le faux serveur OBS existant de `obs.rs` : succès, puis échec sur un port fermé.
- [ ] **Step 2:** RED. **Step 3:** implémenter. **Step 4:** vert. **Step 5: Commit** `feat(engine): pipe de controle (status, tests OBS et MIDI)`

### Task 4: Squelette de l'appli Tauri et design system

**Files:**
- Create `ui/Cargo.toml` (crate `nullctl`, `[[bin]] name = "nullctl"`, dépendance `engine = { path = "../engine" }`), `ui/build.rs`, `ui/tauri.conf.json`, `ui/capabilities/default.json`, `ui/src/main.rs`.
- Create `ui/design/…`, une copie des fichiers `project/` de l'artefact (`tokens.json`, `README.md`, `components/bundle.{js,css}`, `components/*/{README.md,preview.html}`, `assets/Modele/{nullpad.json,README.md}`).
- Create `ui/web/index.html`, `ui/web/css/tokens.css` (généré à la main depuis `tokens.json` : une variable `--<name>` par token, plus `--font-<famille>`), `ui/web/js/bridge.js`, `ui/web/js/app.js`, `ui/web/vendor/{three.module.js,lucide.min.js}`, `ui/web/fonts/` (IBM Plex Sans/Mono, Bagel Fat One, Silkscreen en woff2, licence OFL).
- Modify : `Cargo.toml` (membre `ui`), `.gitignore` (`ui/target`, le cas échéant), `.claude/launch.json` (une config `ui-preview` qui sert `ui/web` en statique, par exemple `npx --yes http-server@14.1.1 ui/web -p 5174 -c-1`).

**Interfaces — Produces :**
- `bridge.js` exporte `invoke(cmd, args)` et `listen(event, cb)`.
  - Avec `window.__TAURI__` présent, ils utilisent l'API Tauri.
  - Sinon, ils servent un **mock** : une config d'exemple (profils DEF, STR), des événements simulés (un `key` toutes les 3 s si `?demo=1`), et `save_config` qui valide grossièrement puis répond `ok`.
- **Polices** : uniquement les fichiers locaux. Le `@import` Google Fonts de `bundle.css` est retiré et remplacé par des `@font-face` locaux. Le README de `ui/design/` signale cette différence avec l'artefact.
- **Coquille statique** de l'écran principal, avec les classes `nc-*` : barre de titre, barre des profils, scène avec le cadre terminal, panneau vide. Fenêtre Tauri de 1440×900 au départ, 1100×720 au minimum, `decorations: false` (barre de titre maison, boutons fenêtre câblés à l'API Tauri).

- [ ] **Step 1: Copier le design system.** Lecture des fichiers de l'artefact, puis vérification que `nullpad.json` se charge et que les noms de pièces `F1`–`F23` et `D1`/`D2` y figurent.
- [ ] **Step 2: Squelette.** `cargo build -p nullctl` passe. Ouvrir `ui-preview` dans le navigateur intégré et faire une capture : la coquille ressemble à `EcranPrincipal`, pad mis à part.
- [ ] **Step 3: Commit** `feat(ui): squelette Tauri et design system nullctl`

### Task 5: Côté Rust de l'interface (commandes)

**Files:** Create `ui/src/{lib.rs, store.rs, native.rs, pipes.rs}`. Test : modules `tests` de chacun.

**Interfaces — Produces** (commandes Tauri, exposées aussi comme fonctions Rust testables) :
- `store::load(path) -> Result<Config, String>` et `store::validate(json: &str) -> Result<(), String>` (avec `engine::config::parse`).
- `store::save_atomic(path, json: &str) -> Result<(), String>` : valide, écrit `config.json.tmp` dans le même dossier, vide le tampon (`sync_all`), puis `std::fs::rename`, qui remplace atomiquement sur le même volume Windows.
- `native::native_keys(keymap: &str) -> BTreeMap<String, String>` : analyse la **couche 0** de `firmware/null_pad.keymap` (inclus par `include_str!`). Elle associe l'`InputId` (`K00`…) à une légende pour chaque `&kp KP_*` ou `&np_00`. Table : `KP_N0`→« 0 »…`KP_N9`→« 9 », `KP_SLASH`→« / », `KP_MULTIPLY`→« * », `KP_MINUS`→« − », `KP_PLUS`→« + », `KP_ENTER`→« ENTRÉE », `KP_DOT`→« . », `KP_NUMLOCK`→« VERR NUM », `np_00`→« 00 ». Les positions `ACT(…)` ne sont pas natives. M1–M3 et la molette gauche ont leur propre table fixe (« P1 », « VOL+ »…).
- `pipes::spawn_events_relay(app)` : un thread qui se connecte à `\\.\pipe\nullctl-events`, relaie chaque ligne en événement Tauri `engine-event` et se reconnecte toutes les secondes s'il est coupé. Il émet `engine-up` et `engine-down`.
- `pipes::control(req) -> Result<Response, String>` : se connecte, envoie, lit la réponse, avec un délai maximal de 3 s.
- Commandes Tauri :
  - `get_config` ;
  - `validate_config(json)` ;
  - `save_config(json)` ;
  - `native_keys` ;
  - `engine_status` (le `Status` du pipe de contrôle) ;
  - `test_obs` / `test_midi` ;
  - `start_engine` (lance `nullctl-engine.exe` depuis le dossier de l'exécutable) ;
  - `config_path`.
- Plugins : `single-instance` (une seconde instance ramène la fenêtre) et `dialog` (sélecteurs de fichiers).

- [ ] **Step 1: Tests qui échouent** :
  - `save_atomic_never_leaves_partial` : une écriture valide puis une invalide ; le fichier garde le premier contenu, sans `.tmp` qui traîne ;
  - `validate_rejects_bad_json_and_rules` ;
  - `native_keys_from_real_keymap` : `K00`→« VERR NUM », `K04`→« 7 », `K16`→« 0 », `K17`→« 00 » ; `K11` et `K19` absents ;
  - `control_roundtrip` contre `engine::ipc::serve_control` sur un nom unique.
- [ ] **Step 2:** RED. **Step 3:** implémenter. **Step 4:** `cargo test --workspace` et clippy → vert.
- [ ] **Step 5: Commit** `feat(ui): commandes (config, touches natives, pipes)`

### Task 6: Modèles purs de l'interface (JS testé)

**Files:** Create `ui/web/js/{catalog.js, legend.js, draft.js, drop.js}` et `ui/web/test/{legend,draft,drop}.test.mjs`.

**Interfaces — Produces :**
- `catalog.js` : `ACTIONS`, la liste des types d'action, chacun avec `{type, label, desc, icon}`. Libellés et icônes Lucide repris de la preview `EcranPrincipal` : Raccourci clavier, Texte, Macro, Lancer, Commande, Son, OBS (qui regroupe les 5 types), Volume et micro, MIDI, Média, Bascule. Plus `TRIGGERS = ["Press","Hold","DoubleTap"]` et leurs libellés FR.
- `legend.js` : `legendFor(binding, natives, sounds) -> {text, kind}`, où `kind` vaut `"action"`, `"native"`, `"free"` ou `"profile"`.
  - Un `label` défini l'emporte.
  - Sinon : `Sound` → l'id en majuscules ; `ObsScene` → la scène ; `Shortcut` → les touches (« CTRL+C ») ; `Volume ±` → « VOL+ » ou « VOL− » ; `Launch` → le nom du fichier sans extension ; `MidiNote` → « N60 » ; `MidiCc` → « CC20 » ; touche native → la légende du firmware ; pas de liaison → « + ».
  - `fitLegend(text) -> [l1, l2?]` : 2 lignes de 6 caractères au plus, coupées à un espace ou un `+`, puis tronquées avec « … ».
- `draft.js` : `createDraft(config, profileId)` renvoie `{get(), set(input, trigger, binding|null), setProfileMeta({name, tag}), dirty(), merged(): Config}`.
  - `merged()` remplace le seul profil concerné dans une **copie** de la config ; les autres profils et les sections globales ne bougent pas.
  - `canBind(input, natives)` vaut `false` pour une touche native et pour M1–M3 (touches de profil du firmware).
- `drop.js` : `actionForDrop(paths, target, natives) -> {action?, warning?, error?}`.
  - Extensions :
    - `.mp3`, `.wav`, `.ogg`, `.flac` → `{type:"Sound", id}`, avec un nouveau `SoundDef` à ajouter (id = nom de fichier en minuscules, `[a-z0-9-]`, rendu unique : `-2`, `-3`…) ;
    - `.exe`, `.lnk` → `Launch{program, args:[]}` ;
    - `.ps1` → `Command{line:"powershell -ExecutionPolicy Bypass -File \"<p>\""}` ;
    - `.bat`, `.cmd` → `Command{line:"\"<p>\""}` ;
    - autre → `Open{target}`.
  - Plusieurs fichiers : seul le premier compte, avec `warning:"Seul le premier fichier est utilisé"`.
  - Cible native, ou M1–M3 → `error` avec le message de la spec §12.5.

- [ ] **Step 1: Tests `node --test` qui échouent**, un par règle ci-dessus, dont :
  - la priorité de `label` ;
  - la coupe « SCENE JEU PRINCIPALE » ;
  - l'unicité des id de son ;
  - le brouillon qui ne touche ni aux autres profils ni à `sound`/`obs`/`midi` ;
  - `dirty()` revenu à `false` si on remet la valeur d'origine.
- [ ] **Step 2:** RED (`node --test ui/web/test/`). **Step 3:** implémenter. **Step 4:** vert.
- [ ] **Step 5: Commit** `feat(ui): modeles legendes, brouillon, depot de fichiers`

### Task 7: Le pad en 3D, rendu à la demande

**Files:** Create `ui/web/js/pad.js`. Modify `ui/design/components/bundle.js` seulement si `NullCtl.pad3d` ne se laisse pas piloter de l'extérieur : préférer un enrobage ; sinon, une modification minimale documentée dans `ui/design/README-ecarts.md`.

**Interfaces — Produces :** `createPad(stageEl, geo, opts) -> pad`, avec :
- `pad.setLegends(map)` : `InputId` → `[l1, l2]` et un `kind` (`free` en gris) ;
- `pad.setLcd({top, tag, bottom})` ;
- `pad.select(input|null)` et `pad.hover(input|null)` ;
- `pad.press(input, down)` : enfoncement d'environ 1,5 mm et lueur violette ;
- `pad.pickAt(x, y) -> input|null`, pour le dépôt de fichiers ;
- `pad.onPick(cb)` ;
- `pad.frames()` : compteur d'images, pour les tests ;
- `pad.dispose()`.

Correspondance pièces → entrées : `F1`–`F20` → `K00`–`K19`, `F21`–`F23` → `M1`–`M3`, `D1` → molette gauche, `D2` → molette droite.

**Règles :**
- On n'appelle `renderer.render` que dans `invalidate()`, regroupé par un seul `requestAnimationFrame` en attente.
- `invalidate()` est appelé uniquement sur : survol, sélection, appui, changement de légendes ou de LCD, redimensionnement (`ResizeObserver`), glitch.
- Sur `document.hidden`, aucun rendu.
- `setPixelRatio(Math.min(devicePixelRatio, 1.5))`.

- [ ] **Step 1:** en mode `?test=1`, `window.__padFrames` expose `pad.frames()`.
- [ ] **Step 2: Implémenter**, puis vérifier dans le navigateur intégré (`ui-preview`) :
  - capture avec légendes et sélection ;
  - `__padFrames` stable sur 2 s sans interaction ;
  - +1 à 3 images après un survol.
- [ ] **Step 3: Commit** `feat(ui): pad 3D a la demande`

### Task 8: Mode vue

**Files:** Create `ui/web/js/{shell.js, viewpanel.js, live.js}`. Modify `app.js`.

**Interfaces — Consumes :** `bridge` (Task 4), `catalog`, `legend` (Task 6), `pad` (Task 7), commandes et événements (Task 5).

**Comportement** (spec §7) :
- **Barre des profils** :
  - emplacements M1–M3 en autocollant (`nc-sticker`) et « + Nouveau profil » ;
  - à droite, en permanence : connexion USB, BLE ou « Pad absent » ; moteur « en marche » ou « arrêté », avec le bouton « Démarrer le moteur » ; bouton « Configurer » ; bouton Réglages.
- **Légendes** du profil affiché, calculées par `legendFor`.
- **Clic sur une touche** : panneau en lecture seule.
  - Il montre l'action de chaque déclencheur (onglets), en libellé et en description.
  - Aucun contrôle d'édition, mais un bouton « Configurer ».
  - Une touche native affiche « Touche native · 7, envoyée par le pad, marche sans nullctl ».
- **`engine-event`** :
  - `key` → `pad.press` ;
  - `action` → une ligne dans la scène (`nc-toast`, 4 s), « K07 → Son : airhorn » ou l'erreur ;
  - `profile` → change le profil affiché et joue le glitch ;
  - `link` → met à jour la pastille ;
  - `config` → met à jour l'état d'enregistrement.
- Moteur arrêté, avec `engine-down` → pastille et bouton.
- Un dépôt de fichier en mode vue est refusé par un toast « Passer en configuration pour modifier ».

- [ ] **Step 1: Vérifier dans le navigateur intégré** avec le mock `?demo=1` :
  - capture du mode vue ;
  - un `key` simulé enfonce bien la touche ;
  - un `profile` simulé joue le glitch puis change les légendes ;
  - un clic ouvre le panneau en lecture seule.
- [ ] **Step 2:** `node --test` et `cargo test --workspace` restent verts.
- [ ] **Step 3: Commit** `feat(ui): mode vue (legendes, appuis en direct, statut)`

### Task 9: Mode configuration, panneau d'édition et dépôt

**Files:** Create `ui/web/js/{config-mode.js, editpanel.js, editors/*.js}`. Modify `app.js`, `pad.js` (survol pendant un dépôt).

**Comportement** (spec §8 et §12.3–6) :
- **« Configurer »** :
  - crée un `draft` sur le profil affiché ;
  - affiche le bandeau « CONFIG // NON ENREGISTRÉ » dans la scène (mono capitales, sur fond `violet-900`) ;
  - passe le cadre de la scène en `violet-500` ;
  - remplace « Configurer » par « Enregistrer » (bouton principal) et « Annuler ».
- **Panneau d'édition** :
  - en tête : le nom de la touche et la mini-grille ;
  - onglets Appui court / Appui long / Double ;
  - action en cours, puis « Changer d'action » (catalogue) et « Retirer l'action » ;
  - éditeur propre à chaque type :
    - `Shortcut` : capture de touches ;
    - `Text` : zone de texte ;
    - `Macro` : liste d'étapes Faire / Attendre, avec ajout, suppression et déplacement ;
    - `Launch` : sélecteur de fichier, arguments (un par ligne), case URL ;
    - `Open` ;
    - `Command` ;
    - `Media` ;
    - `Volume` / `MicVolume` : pas en % ;
    - `VolumeMute` / `MicMute` ;
    - `Toggle` : deux sous-éditeurs « Allumé » et « Éteint », sans nouvelle bascule dedans ;
    - `Sound` : choix dans la bibliothèque ou nouveau fichier, avec volume et comportement au rappui ;
    - `StopSounds`, `SoundVolume`, `SoundMute` ;
    - OBS : scène, source, stream, enregistrement, replay ;
    - `MidiNote` / `MidiCc` ;
  - « Légende personnalisée » : un champ texte qui alimente `label`.
  - Les champs en `nc-field`, `nc-seg`, `nc-slider`, `nc-check` et `nc-file` du design system.
- **Molettes** : leur panneau a trois sections, Sens horaire / Sens anti-horaire / Clic, chacune avec son éditeur.
- **Dépôt de fichier** (événement natif de dépôt de Tauri, avec la position) :
  - pendant le survol, `pad.pickAt` désigne la cible, qui reçoit une lueur ;
  - au lâcher, `actionForDrop` donne l'action, le brouillon est mis à jour, et le panneau s'ouvre sur la cible ;
  - un `warning` ou une `error` s'affiche en toast.
- **« Enregistrer »** :
  1. `validate_config(merged)` : une erreur s'affiche dans le panneau, et on reste en configuration ;
  2. `save_config` ;
  3. attente de l'événement `config`, 3 s au plus :
     - `ok` → « Enregistré », glitch, sortie du mode ;
     - erreur → message, et le brouillon est gardé ;
     - pas de réponse, ou moteur arrêté → « Enregistré, pris en compte au démarrage du moteur », puis sortie du mode.
- **Ce qui se passe hors du chemin normal** :
  - « Annuler » avec un brouillon modifié demande confirmation.
  - Changer de profil affiché ou fermer la fenêtre avec un brouillon modifié ouvre une boîte « Enregistrer / Abandonner / Rester ». Elle est stylée `nc-panel`, et la fermeture passe par l'interception de `close-requested` dans Tauri.
  - Un `profile` du pad pendant la configuration ne change pas l'affichage : un toast dit « Le pad est passé sur STR ».

- [ ] **Step 1:** des tests `node --test` pour la machine à états de la sauvegarde (`config-mode.js` exporte `saveFlow(deps)` avec des dépendances injectées) : les 4 issues, et le brouillon jamais perdu en cas d'erreur.
- [ ] **Step 2:** RED. **Step 3:** implémenter. **Step 4:** vérifier dans le navigateur intégré avec le mock :
  - capture du mode configuration avec le bandeau ;
  - édition d'un son ;
  - un dépôt simulé (le mock expose `__simulateDrop(paths, x, y)`) remplit l'action ;
  - boîte de confirmation au changement de profil.
- [ ] **Step 5:** `node --test`, `cargo test --workspace` et clippy → vert.
- [ ] **Step 6: Commit** `feat(ui): mode configuration a brouillon, editeurs, depot de fichiers`

### Task 10: Profils (créer, renommer, supprimer)

**Files:** Create `ui/web/js/profiles.js`. Modify `app.js`, `shell.js`.

**Comportement :**
- **« + Nouveau profil »** ouvre l'écran plein cadre `CreationProfil`, adapté de sa preview (décor à gauche, usage à droite).
  - Nom en `profile-hero`.
  - Tag de 3 caractères réglable lettre par lettre (▲▼, alphabet `A–Z0–9`), proposé à partir du nom tant qu'on ne l'a pas modifié.
  - Touche de profil M2 ou M3 : simple information, puisque les touches M sont fixées par le firmware (profils 1–3) ; pour 4–8, l'écran dit « atteignable par ZMK Studio ».
  - Point de départ : vide, ou copie du profil affiché.
  - Créer (Entrée) ou Annuler (Échap).
  - **Id** : le plus petit libre entre 1 et 8. S'il n'en reste aucun, le bouton est grisé avec une explication.
  - **Créer** : écriture immédiate (validation puis `save_config`), glitch, toast « Profil … créé · M2 → TAG ».
- **Renommer** (nom et tag) : seulement en mode configuration, dans le brouillon du profil, depuis la tête de la scène.
- **Supprimer** : depuis un menu sur l'emplacement du profil (hors profil 1). Confirmation qui nomme le profil, puis écriture immédiate.

- [ ] **Step 1:** des tests `node --test` pour `suggestTag(name)` (« STREAM NIGHT » → « STN », lettres non A–Z0–9 retirées, complété par `X`) et `nextFreeId(config)`.
- [ ] **Step 2:** RED. **Step 3:** implémenter. **Step 4:** capture de l'écran de création dans le navigateur intégré.
- [ ] **Step 5: Commit** `feat(ui): creation, renommage et suppression de profils`

### Task 11: Écran Réglages

**Files:** Create `ui/web/js/settings.js`. Modify `app.js`.

**Comportement** (spec §9). L'écran remplace la zone scène et panneau, avec un retour par Échap ou une croix. Il a son brouillon propre (`sound`, `obs`, `midi`, `timing` et démarrage avec Windows) et ses boutons Enregistrer / Annuler, qui suivent le même flux que la Task 9.

Rubriques, en onglets verticaux :
- **Audio** :
  - choix de la sortie casque et de VB-Cable : liste des périphériques, donnée par une nouvelle commande `list_audio_outputs` qui réutilise `rodio::cpal` côté Rust ;
  - état VB-Cable : détecté, ou absent avec le lien ;
  - volumes : soundboard, casque, câble ;
  - mode micro, avec l'avertissement.
- **Sons** : tableau de la bibliothèque (nom, fichier, durée si disponible, volume, rappui), ajouter par sélecteur ou par dépôt, retirer. Un son encore utilisé par une liaison ne peut pas être retiré : message qui donne les touches concernées.
- **OBS** : hôte, port, mot de passe masqué, « Tester » (via `test_obs`, résultat en ligne).
- **MIDI** : port, « Tester ».
- **Démarrage** :
  - « Lancer avec Windows » : nouvelles commandes `get_autostart` / `set_autostart`, qui réutilisent `engine::autostart` en pointant vers `nullctl-engine.exe` ;
  - appui long et double appui, en ms, bornés à 50–5000.

- [ ] **Step 1:** tests `node --test` pour `soundInUse(config, id) -> [inputs]`.
- [ ] **Step 2:** RED. **Step 3:** implémenter. **Step 4:** captures des 5 onglets dans le navigateur intégré.
- [ ] **Step 5: Commit** `feat(ui): ecran reglages`

### Task 12: Build, documentation, test « effets coupés »

**Files :**
- Create `ui/README.md` : lancer en développement (`cargo tauri dev` dans `ui/`), construire (`cargo tauri build --no-bundle`, ou `cargo build -p nullctl --release`), préversion navigateur, tests.
- Modify `README.md` racine : la ligne « Interface » devient « ✅ faite, à valider sur le vrai pad » ; ajouter `ui/` dans « Organisation ».
- Modify `engine/README.md` : noms, migration du dossier, menu « Ouvrir nullctl ».

**Étapes :**
- [ ] **Step 1: Test « effets coupés ».** La classe `no-fx` coupe grain, trame, fenêtres du décor, glitch, registre, code-barres et étoile. Capture : tout reste lisible et utilisable.
- [ ] **Step 2: Build release.** `cargo build --release --workspace` produit `nullctl-engine.exe` et `nullctl.exe` dans `target/release`. Le contrôleur lance une fois `nullctl.exe` (avec l'accord d'Alan) pour une capture de la vraie fenêtre.
- [ ] **Step 3: Liste « À tester à la main »** dans `ui/README.md` :
  - appuis en direct avec le vrai pad ;
  - USB et BLE ;
  - enregistrement pris par le moteur ;
  - moteur arrêté puis démarré depuis l'interface ;
  - dépôt d'un `.mp3`, d'un `.exe` et d'un `.ps1` ;
  - fermeture avec un brouillon ;
  - GPU au repos (Gestionnaire des tâches, l'interface à ~0 %) ;
  - migration depuis `%APPDATA%\NULL-PAD`.
- [ ] **Step 4:** `cargo test --workspace`, clippy, `node --test ui/web/test/` → vert.
- [ ] **Step 5: Commit** `docs(ui): build, tests a la main, effets coupes`
