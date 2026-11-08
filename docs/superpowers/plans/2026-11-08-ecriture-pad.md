# Écriture dans le pad (ZMK Studio RPC) — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** depuis le mode configuration de nullctl, avec le pad branché en USB, modifier toute touche, molette, macro de profil et tag dans le pad lui-même, grâce au protocole RPC de ZMK Studio. Il y a un seul « Enregistrer » : le pad d'abord, puis la config.

**Architecture :**
- **Client RPC en Rust** dans la crate `nullctl` : `ui/src/studio/`. Il parle au port série USB CDC-ACM du pad (`serialport`).
- **Trames** : `0xAB` / `0xAC` / `0xAD`.
- **Messages** : protobuf générés à la compilation depuis les `.proto` de `zmk-studio-messages`, copiés dans `ui/proto/` (`prost` / `prost-build`, avec un `protoc` fourni par `protoc-bin-vendored`).
- **Traduction** : une couche pure et testée traduit la keymap du pad en entrées et profils nullctl. Elle sait reconnaître `&kp ACT(p,i)`, les touches natives et les macros `np_pN`.
- **Pilotage** : des commandes Tauri pilotent la lecture (avec déverrouillage), l'écriture (pad d'abord), l'abandon et un cache de la keymap.
- **Interface** : la partie web gagne une partie « pad » dans le brouillon, un sélecteur de touche native, des légendes lues dans le pad, une pastille d'état et l'étape pad de `saveFlow`.
- **Le moteur ne change pas.**

**Tech Stack** (versions stables vérifiées sur crates.io le 2026-11-08) : Rust, `serialport = "4.10"`, `prost = "0.14"`, `prost-build = "0.14"` et `protoc-bin-vendored = "3.3"` en build-dependencies. Côté web : JS en modules ES, tests `node --test`.

**Spec :** `docs/superpowers/specs/2026-11-08-ecriture-pad-design.md` (§1–9 ; le §9 tranche les points de la relecture et **remplace** l'ordre du §5 : le pad d'abord). Complète `2026-11-07-interface-design.md`.

## Global Constraints

- **Protocole épinglé** sur `zmk-studio-messages` @ `6cb4c283e76209d59c45fbcb218800cd19e9339d` (celui de ZMK v0.3.0). Les `.proto` sont copiés dans `ui/proto/` avec leur LICENSE ; le README de ce dossier donne le commit.
- **Trames** : début `0xAB`, échappement `0xAC` (l'octet suivant est pris tel quel), fin `0xAD`. Un octet de valeur `0xAB`, `0xAC` ou `0xAD` dans le message est précédé de `0xAC`.
- **Port** : port série USB de VID `0x1D50` et PID `0x615E`. On l'ouvre à la demande, on le ferme après usage (ZMK Studio doit rester utilisable) et un seul client tourne à la fois. Délai de chaque requête : 3 s. Attente du déverrouillage : 60 s au plus.
- **Verrou** : `keymap` et `behaviors` exigent le pad déverrouillé (`UNLOCK_REQUIRED` sinon). Seuls `get_device_info` et `get_lock_state` passent sans. On attend la notification `lock_state_changed`.
- **Correspondances** :
  - position de touche = `InputId` du moteur (0–28) ;
  - profil `p` = couche d'**indice** `p-1` ;
  - usage d'un `&kp` = `(page << 16) | id` ;
  - action nullctl = `&kp` avec l'usage `ACT(p,i)` = `(0x0C << 16) | (0x0500 + (p-1)*32 + i)` (formule de `firmware/null_pad_codes.h`).
- **Comportements** : on les reconnaît par leur `display_name`, lu une fois par connexion avec `list_all_behaviors` puis `get_behavior_details`. On n'utilise jamais un id écrit en dur.
- **Jamais de `save_changes` si `check_unsaved_changes` était vrai à l'ouverture**, c'est-à-dire avec des changements étrangers en attente (spec §9.5).
- **Garde-fous avant toute écriture** (le pad comme la config) :
  - chaque couche garde au moins un `np_p1` ;
  - la combo de déverrouillage n'est jamais touchée ;
  - un nom de couche fait au plus `max_layer_name_length` caractères ;
  - « Aller au profil » = `np_pN` seulement, jamais `&to` seul.
- **Cache** : `%APPDATA%\nullctl\pad-cache.json` = `{serial, read_at, layers:[{name, bindings:[{behavior, param1, param2}]}], behaviors:{id: display_name}}`. Écriture atomique, comme la config.
- **Tests** : aucun test automatique n'ouvre un vrai port. Ce qui passe par le transport est testé avec un **faux pad** en mémoire (trait `Port: Read + Write`). Les tests sur le vrai pad sont `#[ignore]`.
- Échappement de toute chaîne venue du pad (noms de couches, `display_name`) avant affichage (`esc` ou textContent).
- Commandes cargo sans `rtk`. Commits terminés par `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`. Branche `feat/studio`. Pas de push sans accord.

## Review Focus

1. **Pad débranché ou reverrouillé au milieu d'une écriture** : `discard_changes` quand c'est possible, sinon un message clair. Le brouillon est gardé, la config n'est pas écrite. Tests : Task 2 (faux pad qui coupe) et Task 4.
2. **Changements étrangers dans le pad** (Studio, ou la keymap a changé depuis la lecture) : aucune écriture, une proposition claire. Tests : Task 4.
3. **Trame coupée, octet d'échappement en fin de buffer, réponse d'une autre requête qui arrive entre-temps, notification au milieu** : décodage correct, et rien ne bloque. Tests : Tasks 1 et 2.
4. **Garde-fous** (plus de `np_p1` sur une couche, combo de déverrouillage, nom trop long) : refus avant écriture. Tests : Task 3.
5. **Légendes en mode vue avec un cache venant d'un autre pad (autre numéro de série) ou corrompu** : on retombe sur la table par défaut, sans erreur. Tests : Task 4.

---

### Task 1: Protobuf et trames

**Files:** Create `ui/proto/{studio,core,keymap,behaviors,meta}.proto` + `ui/proto/LICENSE` + `ui/proto/README.md` ; modify `ui/build.rs` (prost-build), `ui/Cargo.toml` ; create `ui/src/studio/{mod.rs, framing.rs}`.

**Interfaces — Produces :**
- `mod pb` (généré) : `zmk::studio::{Request, Response, RequestResponse, Notification}`, `zmk::keymap::…`, `zmk::core::…`, `zmk::behaviors::…`.
- `framing::encode(msg: &[u8]) -> Vec<u8>`.
- `framing::Decoder::new()` et `push(&mut self, bytes: &[u8]) -> Vec<Vec<u8>>` (trames complètes, sans les octets de contrôle). Un octet hors trame est ignoré. Une trame interrompue par un nouveau `SOF` repart de zéro.

- [ ] **Step 1: Tests qui échouent** :
  - `encode` sans échappement ;
  - avec `0xAB`, `0xAC` et `0xAD` dans le message ;
  - décodage en morceaux d'un octet ;
  - `0xAC` en fin de buffer ;
  - deux trames d'un coup ;
  - octets parasites avant `SOF` ;
  - aller-retour d'un `Request { request_id, keymap: get_keymap }` encodé avec prost puis décodé.
- [ ] **Step 2:** RED. **Step 3:** implémenter, avec le build prost et le protoc fourni. **Step 4:** `cargo test -p nullctl` et clippy → vert.
- [ ] **Step 5: Commit** `feat(studio): protobuf ZMK Studio et trames`

### Task 2: Transport et client RPC

**Files:** Create `ui/src/studio/{client.rs, port.rs, fake.rs}`. `fake.rs` est en `#[cfg(test)]`, ou dans un module `testkit` réservé aux tests.

**Interfaces — Produces :**
- `trait Port: Read + Write + Send`.
- `fn find_pad_port() -> Option<String>`, avec `serialport::available_ports()` et un filtre `UsbPort { vid: 0x1D50, pid: 0x615E }`.
- `fn open_pad(name) -> Result<Box<dyn Port>>` : 115200 bauds, délai de lecture de 100 ms (lecture en boucle).
- `struct Client<P: Port>` :
  - `call(req) -> Result<RequestResponse, StudioError>` : `request_id` croissant ; on ignore les réponses à d'autres ids ; délai de 3 s ;
  - `notifications()` : les notifications vues pendant les appels, mises en file ;
  - `wait_unlocked(timeout)` : interroge `get_lock_state` toutes les 500 ms, ou s'arrête dès la notification `lock_state_changed` ;
  - helpers `device_info`, `lock_state`, `behaviors() -> HashMap<u32, String>`, `keymap()`, `set_binding(layer_id, pos, binding)`, `set_layer_name(layer_id, name)`, `check_unsaved()`, `save()`, `discard()`.
- `enum StudioError { Timeout, Locked, Disconnected, Rejected(String), Decode(String) }` : messages en français pour l'interface.
- **Faux pad** (`fake.rs`) : il tient une keymap de 8 couches, celle de `firmware/null_pad.keymap` (positions 0–28), un état de verrou, des changements en attente et un « couper après N octets ». Il répond aux requêtes utilisées, avec des comportements nommés « Key Press », « np_p1 » à « np_p8 », « np_00 », « Studio Unlock » et « Transparent ».

- [ ] **Step 1: Tests qui échouent** (faux pad) :
  - `get_device_info` ;
  - `keymap()` sur un pad verrouillé → `Locked` ;
  - `wait_unlocked` qui réussit après une notification ;
  - réponse d'un autre id ignorée ;
  - délai dépassé ;
  - pad coupé au milieu → `Disconnected` ;
  - `set_binding` puis `save` ;
  - notification reçue au milieu d'un appel → mise en file.
- [ ] **Step 2:** RED. **Step 3:** implémenter. **Step 4:** vert, avec un test `#[ignore] reads_real_pad` qui lit device_info et la keymap d'un vrai pad.
- [ ] **Step 5: Commit** `feat(studio): client RPC et faux pad`

### Task 3: Modèle pur — keymap ↔ entrées, touches natives, garde-fous

**Files:** Create `ui/src/studio/{model.rs, hid.rs}`.

**Interfaces — Produces :**
- `hid::name(usage: u32) -> Option<&'static str>` et `hid::catalog() -> Vec<KeyChoice {usage, label, group}>`.
  - Groupes : Clavier, Pavé numérique, Média et volume, Modificateurs.
  - Les codes viennent de `app/include/dt-bindings/zmk/keys.h` de ZMK v0.3.0 : une table recopiée, avec un commentaire qui cite la source.
  - Les combinaisons (Ctrl, Maj, Alt, Win) passent par les bits de modificateurs implicites de ZMK sur l'usage (`LC(...)` etc., à vérifier dans `keys.h` et `modifiers.h`).
- `enum PadBinding { Action { profile, input }, Native { usage, label }, ProfileMacro(n), Other { behavior: String, p1, p2 } }`.
- `fn decode(keymap, behaviors) -> PadModel { layers: Vec<{id, name, bindings: [PadBinding; 29]}> }`.
- `fn encode(b: &PadBinding, behaviors) -> Result<pb::BehaviorBinding>` : cherche « Key Press » et les `np_pN` par leur nom.
- `fn default_layer(p) -> [PadBinding; 29]` : grille en `Action`, sauf la couche 0 qui reprend le pavé natif du firmware ; `np_p1..3` ; molette gauche en volume ; molette droite en `Action`.
- `fn diff(before: &PadModel, after: &PadModel) -> Vec<PadChange { layer_id, pos?, binding?, name? }>`.
- `fn guard(model) -> Result<(), String>` : chaque couche garde au moins un `np_p1`, la combo de déverrouillage n'est pas touchée, les noms respectent la longueur.

- [ ] **Step 1: Tests qui échouent** :
  - décoder le faux pad donne `K04` = Native « 7 » en couche 0 et `K04` = Action{2,K04} en couche 1 ;
  - aller-retour `encode`/`decode` ;
  - `default_layer` correspond à la keymap du firmware ;
  - `diff` minimal ;
  - `guard` refuse une couche sans `np_p1` ;
  - `hid::name` pour `KP_N7`, `C_VOL_UP` et `LC(C)`.
- [ ] **Step 2:** RED. **Step 3:** implémenter. **Step 4:** vert.
- [ ] **Step 5: Commit** `feat(studio): modele de la keymap du pad`

### Task 4: Service pad et commandes Tauri

**Files:** Create `ui/src/studio/service.rs` et `ui/src/studio/cache.rs` ; modify `ui/src/lib.rs` (commandes).

**Interfaces — Produces** (commandes Tauri, sérialisées en JSON pour le web) :
- `pad_status() -> {present, locked, serial?, cache_at?}`, sans déverrouillage.
- `pad_read(wait_unlock_secs) -> PadModel`.
  - Il déverrouille si nécessaire, en émettant l'événement Tauri `pad-unlock-needed` pour que l'interface affiche « Appuie sur M1 + M3 ».
  - Il lit les comportements et la keymap, refuse quand `check_unsaved` est vrai (erreur `foreign-unsaved`), puis met à jour le cache.
- `pad_cached() -> Option<PadModel>` : seulement si le numéro de série du cache est celui du pad présent, ou si aucun pad n'est branché. Un cache corrompu donne `None`.
- `pad_write(expected: PadModel, target: PadModel) -> Result<PadModel>`.
  1. Relire et comparer avec `expected`. Si elles diffèrent → erreur `changed-elsewhere`.
  2. `guard(target)`.
  3. `diff`, puis les `set_*`, puis `save`.
  4. Si une étape échoue → `discard` et l'erreur.
  5. Si tout réussit → relecture, mise à jour du cache, et renvoi de la nouvelle lecture.
- `pad_discard_foreign()` : `discard_changes` après une confirmation donnée côté interface.
- Toutes les commandes s'exécutent hors du thread de l'interface (`async` + `spawn_blocking`). Un mutex garantit un seul client.

- [ ] **Step 1: Tests qui échouent** (service contre le faux pad, cache dans un dossier temporaire) :
  - lecture d'un pad verrouillé qu'on déverrouille ;
  - `foreign-unsaved` ;
  - `changed-elsewhere` ;
  - écriture qui réussit ;
  - échec au milieu → `discard` appelé ;
  - cache d'un autre numéro de série ignoré ;
  - cache corrompu → `None`.
- [ ] **Step 2:** RED. **Step 3:** implémenter. **Step 4:** `cargo test --workspace` et clippy → vert.
- [ ] **Step 5: Commit** `feat(studio): service pad et commandes`

### Task 5: Interface — pad dans le brouillon, sélecteur natif, légendes, enregistrement

**Files:** Modify `ui/web/js/{app.js, draft.js, config-mode.js, editpanel.js, editors/*, legend.js, shell.js, bridge.js, profiles.js}` ; create `ui/web/js/pad-model.js` (pur) et `ui/web/test/pad.test.mjs`.

**Comportement** (spec §4, §9) :
- **Pastille de la barre**, alimentée par `pad_status` (au démarrage et toutes les 5 s) : « Pad modifiable », « Pad verrouillé », « Pad non branché » ou « Pad pas à jour ».
- **Légendes** : avec une keymap du pad (lue ou en cache), une touche `Native` affiche son libellé, une `Action` affiche la légende nullctl et une `ProfileMacro(n)` affiche « P{n} ». Sinon, c'est la table actuelle.
- **Entrée en mode configuration avec le pad présent** :
  - `pad_read` affiche une modale « Appuie sur M1 + M3 » (annulable) pendant l'événement `pad-unlock-needed` ;
  - si c'est refusé ou annulé, le mode continue sans édition des touches natives, comme aujourd'hui ;
  - si c'est accepté, le brouillon garde `padExpected` et `padTarget`.
- **`canBind`** devient « éditable si (action nullctl) ou (pad modifiable) ». Les entrées natives ont un éditeur **Touche native**, avec le catalogue `hid` (commande `hid_catalog`), une recherche et la capture d'une touche au clavier, plus « Profil N » (`np_pN`). Passer une touche en native retire ses liaisons nullctl dans ce profil. Passer une touche native en action met `Action{p,i}` dans `padTarget`.
- **Glisser-déposer sur une touche native** quand le pad est modifiable : elle devient une action.
- **Renommer le tag** met aussi à jour le nom de la couche dans `padTarget`.
- **Profils** (créer, copier, supprimer) : si le pad est modifiable, la couche est mise à jour (`default_layer`, copie, ou remise à zéro et nom « P0N ») dans la même opération, en passant par `pad_write`. Sinon, l'opération reste dans la config, et la pastille passe à « Pad pas à jour ».
- **`saveFlow`** : si `padTarget` diffère de `padExpected`, on fait `pad_write` **d'abord**.
  - Erreurs gérées : `changed-elsewhere` (proposer de recharger), `foreign-unsaved` (proposer d'abandonner les changements étrangers après confirmation), verrou expiré (redemander M1 + M3).
  - Puis la config, comme aujourd'hui.
  - Si la config échoue après la réussite du pad → « Pad mis à jour, config non enregistrée », et le brouillon est gardé.
- **Mock** (`bridge.js`) : un faux pad pour la préversion dans le navigateur (`?pad=1`), avec le verrou et toutes les erreurs ci-dessus déclenchables.

- [ ] **Step 1: Tests `node --test` qui échouent** :
  - `pad-model.js` : légendes depuis le pad, `canBind` selon l'état du pad, passage native ↔ action dans le brouillon, renommage du tag vers le nom de couche ;
  - `saveFlow` : le pad d'abord, chaque issue d'erreur, config en échec après le pad.
- [ ] **Step 2:** RED. **Step 3:** implémenter. **Step 4:** tests verts, puis une capture dans le navigateur intégré par le contrôleur (`?pad=1`).
- [ ] **Step 5: Commit** `feat(ui): ecriture dans le pad depuis le mode configuration`

### Task 6: Documentation et liste à tester

**Files:** Modify `ui/README.md`, `firmware/README.md` (nullctl remplace ZMK Studio pour les réglages courants ; Studio reste possible), `README.md` racine (la ligne « Écriture dans le pad depuis l'appli » passe à « ✅ faite, à valider sur le vrai pad »).

**À tester sur le vrai pad** (`ui/README.md`) :
- lecture après M1 + M3 ;
- 7 → Copier, puis retour au 7 ;
- molette gauche → défilement ;
- tag renommé, visible sur l'écran ;
- création et suppression d'un profil dans le pad ;
- pad débranché pendant l'écriture ;
- changement fait dans ZMK Studio puis enregistrement dans nullctl (refusé) ;
- reverrouillage après 10 min.

- [ ] **Step 1: Écrire.** **Step 2:** `cargo test --workspace`, clippy et `node --test ui/web/test/*.test.mjs` → vert. **Step 3: Commit** `docs: ecriture dans le pad`
