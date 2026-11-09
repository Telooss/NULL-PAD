# Installeur et mise à jour automatique — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal :** un installeur Windows de NULL/CONTROL (interface + moteur), avec une mise à jour en un clic depuis l'appli, signée, et publiée depuis les Releases GitHub par un script lancé en local.

**Architecture :**
- **Moteur** : une requête de contrôle `quit` et un argument `--quit`, pour qu'on puisse l'arrêter de l'extérieur.
- **Interface** : le plugin Tauri `updater`, piloté côté Rust par deux commandes (`update_check`, `update_install`). Avant l'installation, le moteur est arrêté proprement.
- **Empaquetage** : il vit dans un fichier séparé, `ui/tauri.bundle.json`, passé à `cargo tauri build --config`. Ainsi `cargo build` et `cargo test` ne changent pas.
- **Installeur** : NSIS en mode utilisateur courant. Des hooks NSIS arrêtent le moteur et le relancent. À la désinstallation, ils retirent le démarrage automatique et, si la case est cochée, la config.
- **Publication** : `scripts/release.ps1`.

**Tech Stack :** Tauri 2.12, `tauri-plugin-updater = "2.10"` (vérifier la compatibilité exacte avec tauri 2.12 ; stable actuel 2.14.1), `tauri-cli` 2 (`cargo tauri`), NSIS (téléchargé par tauri-cli), PowerShell 5.1, `gh`.

**Spec :** `docs/superpowers/specs/2026-11-09-installeur-design.md`.

## Global Constraints

- Les exe gardent leurs noms : `nullctl.exe` et `nullctl-engine.exe`, dans le même dossier. L'installeur et son dossier s'appellent « NULL-CONTROL ». Le titre de la fenêtre reste « NULL/CONTROL ».
- Identifiant Tauri inchangé : `dev.nullctl.app`. Il garde le `localStorage` de la webview.
- Installation `currentUser` dans `%LOCALAPPDATA%\Programs\NULL-CONTROL`. Langue NSIS : French.
- Endpoint de mise à jour : `https://github.com/Telooss/NULL-PAD/releases/latest/download/latest.json`. Plateforme `windows-x86_64`.
- Clé privée **jamais dans le dépôt**. Elle est lue depuis `%USERPROFILE%\.nullctl\updater.key`, et son mot de passe depuis `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` (demandé par le script s'il est absent). La clé publique est dans `ui/tauri.bundle.json`.
- Mise à jour : vérification au démarrage puis toutes les 6 h, seulement si le réglage est actif (par défaut : actif). Installation **uniquement** après un clic sur « Installer ».
- Désinstallation :
  - le moteur est arrêté ;
  - la valeur `nullctl` de `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` est retirée ;
  - `%APPDATA%\nullctl` n'est supprimé que si la case « supprimer les données » est cochée. Elle est décochée par défaut.
- `cargo build --workspace`, `cargo test --workspace` et `node --test ui/web/test/*.test.mjs` doivent marcher sans `tauri-cli`, sans installeur et sans clé.
- Aucun test automatique n'installe, ne désinstalle, ne lance un exe, n'écrit le registre réel ou `%APPDATA%`, ni ne contacte GitHub.
- Commandes cargo sans `rtk`. Commits terminés par `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`. Branche `feat/installer`. Pas de push sans accord ; `-Publish` est lancé par Alan.

## Review Focus

1. **Moteur en marche pendant une mise à jour ou une désinstallation.** On le ferme avant de remplacer les fichiers, sinon ils sont verrouillés. Il est relancé après une mise à jour s'il tournait. Tests : Task 1 (`--quit`), Task 3 (hooks, vérifiés à la lecture et en essai manuel).
2. **Signature invalide, réseau coupé ou `latest.json` absent (aucune release).** Pas d'installation, un message clair, et l'appli reste utilisable. Tests : Task 2.
3. **Clic sur « Installer » alors qu'un brouillon de config n'est pas enregistré.** Demander confirmation, comme pour fermer l'appli. Tests : Task 2.
4. **Script de release sur un arbre sale, une version qui n'est pas X.Y.Z, ou une version qui n'est pas plus grande que l'actuelle.** Refus avant toute modification. Tests : Task 4.
5. **Désinstallation avec la case décochée.** La config reste intacte. Vérifié en essai manuel (Task 5).

---

### Task 1: Moteur — requête `quit` et argument `--quit`

**Files :** modifier `engine/src/ipc.rs` (`Request::Quit`), `engine/src/main.rs` (traiter `Quit` comme le « Quitter » du tray ; argument `--quit`), `engine/README.md`.

**Interfaces — Produces :**
- `Request::Quit` sérialisé `{"cmd":"quit"}`, sur le modèle des requêtes existantes. Réponse `ok`, puis arrêt propre : même chemin que le menu « Quitter » du tray, sons et MIDI fermés.
- `nullctl-engine.exe --quit` :
  - se connecte à `CONTROL_PIPE` et envoie `quit` ;
  - attend que le pipe disparaisse, 5 s au plus ;
  - code de sortie 0 si le moteur est arrêté ou n'était pas lancé, 1 si le délai est dépassé.
  - Il ne démarre **jamais** de moteur.
- Le client peut vérifier que le serveur est bien `nullctl-engine.exe`, comme l'interface le fait (`verify_server_named`). C'est optionnel : la DACL limite déjà le pipe à l'utilisateur.

- [ ] **Step 1 : tests qui échouent.**
  - Aller-retour serde de `Request::Quit`.
  - Un `serve_control` de test sur un nom de pipe unique : `quit` renvoie ok et déclenche le callback d'arrêt.
  - `--quit` sans moteur → code 0, retour immédiat.
- [ ] **Step 2 :** RED. **Step 3 :** implémenter. **Step 4 :** `cargo test --workspace` et clippy → vert.
- [ ] **Step 5 : Commit** `feat(engine): requete quit et --quit`.

### Task 2: Interface — vérification et installation des mises à jour

**Files :**
- modifier `ui/Cargo.toml` (`tauri-plugin-updater`) et `ui/src/lib.rs` (plugin, commandes) ;
- créer `ui/src/update.rs` ;
- modifier `ui/tauri.conf.json` (`plugins.updater` : endpoint et pubkey de développement vide, voir Step 3) et `ui/capabilities` si nécessaire ;
- côté web : modifier `ui/web/js/{app.js, shell.js, settings.js, settings-model.js, bridge.js}` ;
- créer `ui/web/js/update.js` (pur) et `ui/web/test/update.test.mjs`.

**Interfaces — Produces :**
- `update_check() -> Result<Option<{version, notes, date}>, String>`.
  - Rien de plus récent → `None`.
  - Réseau coupé ou endpoint absent → `Err("network: …")`.
  - Jamais de panique sans release.
- `update_install() -> Result<(), String>`. Il :
  1. télécharge, en émettant l'événement `update-progress {downloaded, total}` ;
  2. vérifie la signature (fait par le plugin) ;
  3. arrête le moteur via `pipes::control("quit")` et attend que le pipe disparaisse (5 s) ;
  4. installe puis redémarre l'appli.
  - En cas d'échec avant l'installation, le moteur n'est pas arrêté, ou il est relancé s'il l'a été.
  - Codes d'erreur : `network:`, `signature:`, `engine:`, `install:`.
- Web :
  - `update.js` pur : `shouldCheck(lastCheck, now, enabled)` (démarrage, puis toutes les 6 h) et `describe(err)` (messages en français).
  - Un bandeau ou toast « Mise à jour X disponible » avec les notes, un bouton « Installer » et une barre de progression, dans le design system existant. Il se ferme avec « Plus tard ».
  - Si un brouillon de config n'est pas enregistré, `confirmBox` avant d'installer, comme à la fermeture.
  - Réglage « Vérifier les mises à jour » dans les réglages, en `localStorage` avec try/catch, actif par défaut.
  - Mock `bridge.js` : `?update=1` (une version 9.9.9 disponible, progression simulée) et `?update=err`.
- La clé publique de production n'existe pas encore. En développement, `tauri.conf.json` reçoit un `plugins.updater` minimal qui compile. La vraie clé publique va dans `ui/tauri.bundle.json` (Task 3). Si le plugin exige une pubkey non vide au démarrage, on ne l'enregistre qu'en build de release (`cfg!(debug_assertions)` ou présence de la config), et `update_check` renvoie `None` en développement. Choisir le plus simple qui compile, et le documenter.

- [ ] **Step 1 : tests qui échouent.**
  - `node` : `shouldCheck` (au démarrage, à 5 h 59 non, à 6 h oui, réglage désactivé → jamais) et `describe` pour chaque code.
  - Rust : si la logique d'arrêt du moteur avant l'installation est extraite en fonction pure (`stop_engine_then(install, stop, start)`), la tester avec des closures : un échec de l'installation relance le moteur.
- [ ] **Step 2 :** RED. **Step 3 :** implémenter. **Step 4 :** tests verts, puis capture dans le navigateur intégré par le contrôleur (`?update=1`).
- [ ] **Step 5 : Commit** `feat(ui): mise a jour depuis l'appli`.

### Task 3: Empaquetage — installeur NSIS, moteur embarqué, hooks

**Files :** créer `ui/tauri.bundle.json`, `ui/nsis/hooks.nsh` et `ui/binaries/.gitignore` (le moteur compilé y est copié, jamais commité).

**Contenu de `ui/tauri.bundle.json`** (fusionné par `cargo tauri build --config tauri.bundle.json`) :
- `productName: "NULL-CONTROL"` et `mainBinaryName: "nullctl"`. Vérifier que l'exe installé s'appelle bien `nullctl.exe`.
- `bundle` :
  - `active: true`, `targets: ["nsis"]`, `createUpdaterArtifacts: true` ;
  - `windows.nsis` : `installMode: "currentUser"`, `languages: ["French"]`, `installerHooks: "nsis/hooks.nsh"` ;
  - le moteur via `externalBin: ["binaries/nullctl-engine"]` (fichier `binaries/nullctl-engine-x86_64-pc-windows-msvc.exe`, installé sous le nom `nullctl-engine.exe` à côté de `nullctl.exe`). Sinon `resources`, si `externalBin` ne garde pas ce nom.
- `plugins.updater` : `endpoints` (cf. Global Constraints), `pubkey` (clé publique d'Alan, ajoutée à la fin de la Task 4) et `windows.installMode: "passive"`.

**Hooks NSIS** (`NSIS_HOOK_PREINSTALL`, `NSIS_HOOK_POSTINSTALL`, `NSIS_HOOK_PREUNINSTALL`, `NSIS_HOOK_POSTUNINSTALL` ; vérifier les noms et la variable de la case « supprimer les données » dans le template `installer.nsi` de tauri-bundler, version utilisée) :
- PREINSTALL : si `$INSTDIR\nullctl-engine.exe` existe, noter si le moteur tourne (`tasklist` ou pipe), lancer `"$INSTDIR\nullctl-engine.exe" --quit`, puis `taskkill /F /IM nullctl-engine.exe` en dernier recours.
- POSTINSTALL : relancer le moteur s'il tournait avant. Le démarrage automatique existant pointe déjà vers `$INSTDIR\nullctl-engine.exe`, et ce chemin ne change pas.
- PREUNINSTALL : `--quit`, puis `taskkill` en dernier recours ; `DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "nullctl"`.
- POSTUNINSTALL : si la case de suppression des données est cochée, `RMDir /r "$APPDATA\nullctl"`. Sinon, ne rien toucher.

- [ ] **Step 1 :** installer `tauri-cli` si absent (`cargo install tauri-cli --version "^2" --locked`). Le contrôleur demande l'accord d'Alan avant, car cela prend plusieurs minutes.
- [ ] **Step 2 :** écrire la config et les hooks.
- [ ] **Step 3 : build sans signature**, pour la structure : `cargo build -p engine --release`, copier le moteur dans `ui/binaries/`, puis `cargo tauri build --config tauri.bundle.json` avec `createUpdaterArtifacts` désactivé, via une surcharge en ligne de commande ou un second petit fichier. Vérifier :
  - l'installeur existe ;
  - avec 7-Zip s'il est disponible, ou par la liste des fichiers du bundle, il contient `nullctl.exe` et `nullctl-engine.exe` ;
  - le dossier d'installation par défaut est bien celui des Global Constraints.
  - **Ne pas lancer l'installeur.**
- [ ] **Step 4 :** `cargo build --workspace` et `cargo test --workspace` toujours verts sans `ui/binaries`.
- [ ] **Step 5 : Commit** `feat: installeur NSIS`.

### Task 4: Script de publication

**Files :** créer `scripts/release.ps1` et `scripts/release-lib.ps1` (fonctions pures), plus `scripts/release.tests.ps1` (Pester s'il est installé, sinon un script d'assertions simple lancé par `powershell -File`).

**Interfaces — Produces :** `scripts\release.ps1 -Version X.Y.Z [-Notes "…"] [-Publish]`, qui enchaîne :
1. **Refus** si : la version n'est pas `^\d+\.\d+\.\d+$`, elle n'est pas strictement plus grande que celle de `ui/tauri.conf.json`, l'arbre git est sale, la branche n'est pas `main`, ou la clé est absente.
2. Mise à jour de la version dans `ui/tauri.conf.json`, `ui/Cargo.toml` et `engine/Cargo.toml`, puis `cargo update -w`.
3. `cargo test --workspace`, clippy et `node --test` : arrêt au premier échec.
4. Build du moteur en release, copie dans `ui/binaries/`, puis `cargo tauri build --config tauri.bundle.json`, avec la clé lue depuis le fichier et le mot de passe depuis l'environnement ou demandé (`Read-Host -AsSecureString`).
5. Écriture de `latest.json` : `{version, notes, pub_date (RFC 3339 UTC), platforms: {"windows-x86_64": {signature: <contenu du .sig>, url: "https://github.com/Telooss/NULL-PAD/releases/download/vX.Y.Z/<nom de l'installeur>"}}}`.
6. Commit `release: vX.Y.Z` et tag `vX.Y.Z`.
7. Avec `-Publish` seulement : `git push`, `git push origin vX.Y.Z`, puis `gh release create vX.Y.Z <setup.exe> <setup.exe.sig> latest.json --title "NULL/CONTROL X.Y.Z" --notes …`.

Fonctions pures dans `release-lib.ps1`, toutes testées : `Test-Version`, `Compare-Version`, `Set-Versions` (sur des copies temporaires des trois fichiers), `New-LatestJson`.

- [ ] **Step 1 :** tests des fonctions pures (version invalide, version non supérieure, remplacement exact dans les trois fichiers sans toucher aux versions des dépendances, `latest.json` conforme).
- [ ] **Step 2 :** RED. **Step 3 :** implémenter. **Step 4 :** tests verts. Lancer `release.ps1` **à blanc** n'est pas demandé : il modifie le dépôt.
- [ ] **Step 5 : Commit** `feat: script de publication`.
- [ ] **Step 6 (Alan, avec le contrôleur) :**
  - générer la clé : `cargo tauri signer generate -w "$env:USERPROFILE\.nullctl\updater.key"`, avec le mot de passe choisi par Alan ;
  - copier la clé publique dans `ui/tauri.bundle.json` ;
  - commit `chore: cle publique de mise a jour`.

### Task 5: Documentation et essai manuel

**Files :** modifier `README.md` (installer depuis la page Releases ; ligne « Installeur et mise à jour » dans le tableau), `ui/README.md` (mise à jour, réglage, mock `?update=1`), `docs/RELEASE.md` (à créer : la clé, comment la sauvegarder, `release.ps1`, ce qui se passe en cas de perte de la clé), et la ligne « Mise à jour du firmware depuis nullctl » en axe d'amélioration dans `README.md`.

**Essai manuel**, avec l'accord d'Alan, car il touche le système :
- installer la version construite ;
- lancer nullctl et le moteur ;
- désinstaller avec la case décochée (la config reste) ;
- réinstaller : la config est retrouvée.

La vraie mise à jour de bout en bout (0.1.x → 0.2.0 depuis l'appli) se teste à la première publication.

- [ ] **Step 1 :** écrire. **Step 2 :** `cargo test --workspace`, clippy et `node --test` → vert. **Step 3 : Commit** `docs: installeur et publication`.
