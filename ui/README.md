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
| `?nofx=1` | mode « effets coupés » (aussi : Réglages > Démarrage > « Couper les effets », gardé dans le navigateur) |

Effets coupés : plus de grain, de trame de points, de fenêtres de décor, de ruban, de textes de registre, de code-barres, d'étoile, ni de glitch. Tout le reste fonctionne pareil.

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
| `connect-src ipc: http://ipc.localhost` | appels Tauri vers le Rust (`invoke`, événements) |

## À tester à la main

- [ ] Appuis en direct avec le vrai pad (la touche s'allume, le profil suit le pad)
- [ ] USB, puis BLE (la pastille de lien change)
- [ ] Enregistrement d'une touche : la prochaine touche pressée est prise par le moteur, pas exécutée
- [ ] Moteur arrêté, puis démarré depuis l'interface (bouton « Démarrer le moteur »)
- [ ] Dépôt d'un `.mp3`, d'un `.exe` et d'un `.ps1` sur une touche
- [ ] Fermeture avec un brouillon non enregistré (la confirmation apparaît)
- [ ] GPU au repos : Gestionnaire des tâches, l'interface reste à ~0 %
- [ ] Migration depuis `%APPDATA%\NULL-PAD` (config reprise dans `%APPDATA%\nullctl`)
- [ ] Mode « effets coupés » : tout reste lisible et utilisable
