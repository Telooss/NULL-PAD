# Interface NULL/CONTROL (`nullctl`)

Fenêtre Tauri 2 : une page web (`web/`, sans étape de build, three.js pour le pad en 3D) et un petit côté Rust (`src/`) qui parle au moteur par deux tubes nommés (voir [../engine/README.md](../engine/README.md)).

## Développement

```powershell
cd ui
cargo tauri dev            # fenêtre avec la page de web/
# ou, sans cargo-tauri :
cargo build -p nullctl     # target\debug\nullctl.exe
```

## Version finale

```powershell
cargo build --release --workspace
# -> target\release\nullctl.exe  (interface)
# -> target\release\nullctl-engine.exe  (moteur)
```

Les deux exécutables vont ensemble : le menu de l'icône du moteur lance `nullctl.exe` situé à côté.

## Préversion dans un navigateur

Avec un faux moteur (le « mock » de `web/js/bridge.js`), sans Tauri :

```powershell
npx --yes http-server@14.1.1 ui/web -p 5174 -c-1
```

Puis ouvrir `http://localhost:5174/`. Paramètres d'URL :

| Paramètre | Effet |
|---|---|
| `?demo=1` | données de démonstration (profils, événements de touches) |
| `?test=1` | expose des aides de test (`__padFrames`, `__layoutOk`) |
| `?pad=1` | faux pad branché et déverrouillé (touches natives modifiables) |
| `?pad=1&padlock=1` | faux pad verrouillé : « Configurer » demande M1 + M3 (déverrouillage au bout de 2 s, ou « Annuler ») |
| `?pad=1&paderr=changed-elsewhere` | la première écriture du pad échoue : keymap modifié ailleurs (« Recharger ») |
| `?pad=1&paderr=foreign-unsaved` | idem : changements non enregistrés faits dans ZMK Studio (« Abandonner les changements du pad ») |
| `?pad=1&paderr=disconnected` | idem : pad débranché pendant l'écriture |
| `?pad=absent` | pas de pad : natives en lecture seule, pastille « Pad non branché » |
| `?nofx=1` | mode « effets coupés » (aussi : Réglages > Démarrage > « Couper les effets », gardé dans le navigateur) |

Effets coupés : plus de grain, de trame de points, de fenêtres de décor, de ruban, de textes de registre, de code-barres, d'étoile, ni de glitch. Tout le reste fonctionne pareil.

## Écriture dans le pad

Depuis « Configurer », nullctl lit et écrit le keymap ZMK du pad en USB (protocole ZMK Studio RPC, `src/studio/`). Pour démarrer : brancher le pad en USB, entrer dans « Configurer », puis appuyer sur **M1 + M3** quand l'appli le demande. Le pad se reverrouille au débranchement ou après 10 min sans activité.

**Ce qui est modifiable** : chaque touche, les deux molettes (rotation et clic), les touches de profil M1–M3 (« Profil N ») et le tag affiché sur l'écran.

Une touche est soit **native** (vraie touche clavier, marche sans logiciel), soit une **action** gérée par nullctl (« Rendre à nullctl »).

**Enregistrer** écrit le pad d'abord, puis la config. Si le pad échoue, rien n'est écrit et le brouillon est gardé.

| Pastille | Sens |
|---|---|
| Pad modifiable | pad lu et déverrouillé |
| Pad verrouillé | M1 + M3 attendu |
| Pad non branché | pas de pad : natives en lecture seule |
| Pad pas à jour | le pad ne suit pas la config (voir plus bas) |
| Pad occupé | une lecture ou une écriture est en cours |

**Changements venus d'ailleurs**
- Changements non enregistrés faits dans ZMK Studio : nullctl refuse d'écrire et propose de les abandonner.
- Keymap modifié depuis la lecture : nullctl propose « Recharger ».

**Sans le pad**, les touches natives sont en lecture seule. Les légendes viennent de la dernière lecture, gardée dans `%APPDATA%
ullctl\pad-cache.json`. Créer ou supprimer un profil ne touche alors que la config, et la pastille passe à « Pad pas à jour ».

**ZMK Studio** reste utilisable, mais pas en même temps que nullctl : un seul programme à la fois peut parler au pad.

## Tests

```powershell
node --test ui/web/test/*.test.mjs
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

`autostart_roundtrip` (moteur) écrit le vrai registre : il est ignoré par défaut (`cargo test -p engine -- --ignored autostart_roundtrip` pour le lancer).

## Politique de sécurité du contenu (CSP)

Définie dans `tauri.conf.json` :

| Source | Pourquoi |
|---|---|
| `default-src 'self'` | tout vient du dossier `web/` embarqué |
| `script-src 'self'` | `js/`, `vendor/` (three.js, lucide) ; aucun `<script>` en ligne, ni `eval`, ni `new Function` |
| `style-src 'self' 'unsafe-inline'` | feuilles de `css/` ; le code pose des attributs `style` (positions, largeurs) |
| `img-src 'self' data: blob:` | images locales ; `data:` pour le grain en SVG des feuilles de style ; `blob:` par précaution pour les textures |
| `font-src 'self'` | polices de `web/fonts` |
| `connect-src 'self' ipc: http://ipc.localhost` | `'self'` : `fetch('assets/nullpad.json')` (géométrie du pad) ; `ipc:` et `http://ipc.localhost` : appels Tauri vers le Rust (`invoke`, événements) |

Vérifié sans autre besoin : scripts de module (`js/*.js`, `vendor/three.module.js`) en `script-src 'self'` ; polices en `font-src` ; textures three.js tirées de canvas (aucune URL, pas de CSP) ; SVG de grain en `data:` (`img-src`).

OBS : la scène et la source sont du texte libre (lire la liste depuis OBS n'est pas implémenté).

## À tester à la main

Écriture dans le pad (vrai pad) :

- [ ] Lecture après M1 + M3
- [ ] Touche 7 → Copier, puis retour au 7
- [ ] Molette gauche → défilement
- [ ] Tag renommé, visible sur l'écran
- [ ] Création et suppression d'un profil dans le pad
- [ ] Pad débranché pendant l'écriture
- [ ] Changement fait dans ZMK Studio puis enregistrement dans nullctl (refusé)
- [ ] Reverrouillage après 10 min

Le reste :

- [ ] Appuis en direct avec le vrai pad (la touche s'allume, le profil suit le pad)
- [ ] USB, puis BLE (la pastille de lien change)
- [ ] Enregistrement d'une touche : la prochaine touche pressée est prise par le moteur, pas exécutée
- [ ] Moteur arrêté, puis démarré depuis l'interface (bouton « Démarrer le moteur »)
- [ ] Dépôt d'un `.mp3`, d'un `.exe` et d'un `.ps1` sur une touche
- [ ] Fermeture avec un brouillon non enregistré (la confirmation apparaît)
- [ ] GPU au repos : Gestionnaire des tâches, l'interface reste à ~0 %
- [ ] Migration depuis `%APPDATA%\NULL-PAD` (config reprise dans `%APPDATA%\nullctl`)
- [ ] Mode « effets coupés » : tout reste lisible et utilisable
