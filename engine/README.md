# Moteur de NULL/PAD

Programme Windows 11 qui tourne en arrière-plan (icône dans la zone de notification). Le NULL/PAD envoie des codes HID « Consommateur » réservés ; le moteur les lit via Raw Input, retrouve la liaison configurée (profil + touche + type d'appui) et exécute l'action : raccourci, texte, macro, lancement d'un programme, ouverture d'un lien, commande PowerShell, touche média, volume, micro.

- Une action à appui simple part à l'appui, pas au relâchement.
- Une config invalide ne remplace jamais la dernière config valide.
- Le fichier de config est rechargé automatiquement quand il change.

## Compiler et lancer

```powershell
cargo build -p engine --release     # produit target\release\nullpad.exe
cargo run -p engine --release       # lance le moteur (icône dans la zone de notification)
cargo test -p engine
cargo clippy -p engine --all-targets -- -D warnings
```

Au tout premier lancement (pas de config), le moteur écrit une config par défaut et active le démarrage avec Windows. Une seule instance tourne à la fois.

## Mode simulation

`--simulate` remplace le NULL/PAD par l'entrée standard : une ligne = un événement. Les actions sont réellement exécutées (sur votre bureau !) et les logs s'affichent dans la console. À la fin de l'entrée (Ctrl+Z puis Entrée, ou fin du pipe), le moteur attend les échéances en cours puis s'arrête.

| Ligne | Effet |
|---|---|
| `down <profil> <ENTREE>` | appui sur la touche |
| `up <profil> <ENTREE>` | relâchement |
| `announce <profil>` | le NULL/PAD annonce son profil actif |

`<profil>` va de 1 à 8 ; `<ENTREE>` est un nom de la table plus bas (`K00`…`K19`, `M1`…`M3`, `ENC_L_CW`…).

```powershell
cargo run -p engine -- --simulate                    # config par défaut, saisie au clavier
"down 1 K00`nup 1 K00" | cargo run -p engine -- --simulate
# appui long : attendre plus de hold_ms (300 ms) entre `down` et `up`
# double appui : down/up/down/up en moins de double_ms (250 ms)
```

`--config <chemin>` utilise un autre fichier de config (pratique avec `--simulate` pour ne pas toucher à la vraie config) :

```powershell
nullpad.exe --simulate --config C:\temp\test.json
```

## Fichier de config

Emplacement : `%APPDATA%\NULL-PAD\config.json` (le menu de l'icône ouvre le dossier).

Règles : `version` vaut 1 ; profils 1 à 8, ids uniques ; `tag` = exactement 3 caractères `A-Z0-9` ; une seule liaison par (touche, déclencheur) dans un profil ; `delta` de volume entre -100 et 100 ; un `Toggle` ne peut pas être imbriqué ni placé dans une `Macro`.

Déclencheurs : `Press` (appui), `Hold` (appui maintenu `hold_ms`), `DoubleTap` (deux appuis en moins de `double_ms`). Si une touche a un `Hold` ou un `DoubleTap`, son `Press` est retardé jusqu'à ce que l'on sache lequel des cas s'applique.

Exemple complet (le JSON n'admet pas les commentaires : les explications sont dans la liste qui suit) :

```json
{
  "version": 1,
  "timing": { "hold_ms": 300, "double_ms": 250 },
  "profiles": [
    {
      "id": 1,
      "name": "Bureau",
      "tag": "BUR",
      "bindings": [
        { "input": "K00", "trigger": "Press",
          "action": { "type": "Shortcut", "keys": "Ctrl+Shift+Esc" } },
        { "input": "K01", "trigger": "Press",
          "action": { "type": "Text", "text": "Bonjour !" } },
        { "input": "K02", "trigger": "Press",
          "action": { "type": "Macro", "steps": [
            { "type": "Do", "action": { "type": "Shortcut", "keys": "Win+R" } },
            { "type": "Wait", "ms": 400 },
            { "type": "Do", "action": { "type": "Text", "text": "calc" } },
            { "type": "Do", "action": { "type": "Shortcut", "keys": "Enter" } } ] } },
        { "input": "K03", "trigger": "Press",
          "action": { "type": "Launch", "program": "notepad.exe", "args": [] } },
        { "input": "K04", "trigger": "Press",
          "action": { "type": "Open", "target": "https://example.com" } },
        { "input": "K05", "trigger": "Press",
          "action": { "type": "Command", "line": "Get-Date | Out-File $env:TEMP\\date.txt" } },
        { "input": "K06", "trigger": "Press",
          "action": { "type": "Media", "key": "PlayPause" } },
        { "input": "K07", "trigger": "Hold",
          "action": { "type": "Media", "key": "Next" } },
        { "input": "K07", "trigger": "Press",
          "action": { "type": "Media", "key": "Prev" } },
        { "input": "K08", "trigger": "DoubleTap",
          "action": { "type": "VolumeMute" } },
        { "input": "K09", "trigger": "Press",
          "action": { "type": "Toggle",
            "on":  { "type": "MicMute" },
            "off": { "type": "Shortcut", "keys": "Ctrl+Alt+M" } } },
        { "input": "ENC_L_CW",    "trigger": "Press", "action": { "type": "Volume", "delta": 2 } },
        { "input": "ENC_L_CCW",   "trigger": "Press", "action": { "type": "Volume", "delta": -2 } },
        { "input": "ENC_L_PRESS", "trigger": "Press", "action": { "type": "VolumeMute" } },
        { "input": "ENC_R_CW",    "trigger": "Press", "action": { "type": "MicVolume", "delta": 2 } },
        { "input": "ENC_R_CCW",   "trigger": "Press", "action": { "type": "MicVolume", "delta": -2 } },
        { "input": "ENC_R_PRESS", "trigger": "Press", "action": { "type": "MicMute" } }
      ]
    }
  ]
}
```

Types d'action :

- `Shortcut` : `keys` est une combinaison séparée par `+`. Touches : `Ctrl`, `Shift`, `Alt`, `Win`, `Enter`, `Esc`, `Tab`, `Space`, `Backspace`, `Delete`, `Home`, `End`, `PageUp`, `PageDown`, `Up`, `Down`, `Left`, `Right`, `F1`…`F24`, une lettre ou un chiffre.
- `Text` : tape le texte.
- `Macro` : suite d'étapes `Do` (une action) et `Wait` (`ms` de pause).
- `Launch` : démarre `program` avec `args`, sans attendre.
- `Open` : ouvre un fichier, un dossier ou un lien avec le programme par défaut.
- `Command` : une ligne PowerShell, exécutée masquée, sans attendre.
- `Media` : `PlayPause`, `Next`, `Prev` ou `Stop`.
- `Volume` / `MicVolume` : change le volume de sortie / du micro de `delta` points (en %). `VolumeMute` / `MicMute` : bascule le mutisme.
- `Toggle` : alterne entre `on` et `off` à chaque déclenchement (le premier déclenchement exécute `on`).

## Soundboard

Le moteur joue des sons (mp3, wav, ogg, flac, 5 minutes au plus) dans le casque et dans un micro virtuel, [VB-Cable](https://vb-audio.com/Cable/). Section `sound` de la config (optionnelle, `version` reste 1) :

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

- `*_device` : `null` = périphérique par défaut (casque, micro) ou détection automatique de VB-Cable (sortie dont le nom contient `CABLE Input`) ; sinon une partie du nom, sans tenir compte de la casse.
- Volumes et gain : entiers de 0 à 100. Volume final d'un son = `master_volume` × `volume`/100 × volume de la sortie/100. `monitor_volume: 0` = ne pas s'entendre.
- `on_repeat` : `Restart` (coupe et relance) ou `Stop` (un rappui pendant la lecture l'arrête).
- Un fichier manquant ou illisible n'invalide pas la config : le son est sauté et signalé dans les logs.
- Actions : `{ "type": "Sound", "id": "airhorn" }`, `{ "type": "StopSounds" }`, `{ "type": "SoundVolume", "delta": 5 }` (volume général, agit en direct sur les sons en cours), `{ "type": "SoundMute" }`.

VB-Cable : l'installer, puis choisir « CABLE Output » comme micro dans le jeu, Discord ou OBS. Si VB-Cable est absent, l'icône de la zone de notification propose de l'installer (état lu au démarrage).

Mode micro (`mic_mode`) :

- `External` (défaut) : le logiciel ne touche pas à la voix. Pour que les autres entendent aussi ta voix dans « CABLE Output », brancher le vrai micro sur VB-Cable avec « Écouter ce périphérique » de Windows ou avec l'interface audio. `MicVolume` / `MicMute` règlent le micro Windows, comme avant.
- `App` : le moteur mélange lui-même le micro et les sons vers VB-Cable. Latence d'environ 25–40 ms sur la voix ; micro muet si le moteur est fermé ; ne pas activer en plus « Écouter ce périphérique », sinon le micro est doublé. `MicVolume` / `MicMute` règlent alors le gain et la sourdine du micro de l'appli, et `master_volume` s'applique aussi à lui.

Le volume réglé à la molette (général ou gain micro) vit en mémoire ; la valeur du fichier s'applique au démarrage et chaque fois qu'elle change dans le fichier.

## Molettes

Par défaut, la molette gauche règle le volume Windows et la droite le micro. Pour piloter le volume de la soundboard avec une molette, la lier à `SoundVolume` :

```json
{ "input": "ENC_L_CW",    "trigger": "Press", "action": { "type": "SoundVolume", "delta": 5 } },
{ "input": "ENC_L_CCW",   "trigger": "Press", "action": { "type": "SoundVolume", "delta": -5 } },
{ "input": "ENC_L_PRESS", "trigger": "Press", "action": { "type": "SoundMute" } }
```

## Table des codes

Les entrées :

| Nom | n° | Rôle |
|---|---|---|
| `K00`…`K19` | 0–19 | grille 5 lignes × 4 colonnes, de gauche à droite puis de haut en bas : n° = ligne × 4 + colonne |
| `M1`, `M2`, `M3` | 20–22 | touches M |
| `ENC_L_CW`, `ENC_L_CCW`, `ENC_L_PRESS` | 23, 24, 25 | encodeur gauche : horaire, anti-horaire, appui |
| `ENC_R_CW`, `ENC_R_CCW`, `ENC_R_PRESS` | 26, 27, 28 | encodeur droit : horaire, anti-horaire, appui |

Codes HID Consommateur (usages) envoyés par le NULL/PAD :

- Action : `0x0500 + (profil - 1) × 32 + n°` (profil 1 à 8). Exemple : profil 2, `K03` = `0x0500 + 32 + 3` = `0x0523`.
- Annonce de profil : `0x0600 + profil`. Exemple : profil 3 = `0x0603`.

## Logs

Dossier `%APPDATA%\NULL-PAD\logs`, fichier `engine.log` (rotation quotidienne). En `--simulate`, les logs sont aussi affichés dans la console. Les erreurs de config, d'action et de démarrage automatique y sont consignées.

## À tester à la main

Le test automatique (`cargo test -p engine`, dont `tests/e2e.rs`) couvre appui, appui long, double appui, bascule et macro avec attente, uniquement avec des commandes qui créent des fichiers. Le reste agit sur le bureau et reste à vérifier à la main, avec `--simulate` ou le vrai NULL/PAD :

- [ ] Raccourci (ex. `Ctrl+Shift+Esc` ouvre le gestionnaire des tâches)
- [ ] Texte (tapé dans le bloc-notes)
- [ ] Média (lecture/pause, suivant, précédent pendant une musique)
- [ ] Volume + et - ; mute du son
- [ ] Volume micro + et - ; mute du micro (voir Paramètres > Son)
- [ ] Lancer (`notepad.exe` s'ouvre)
- [ ] Ouvrir (`https://example.com` s'ouvre dans le navigateur)
- [ ] Icône de la zone de notification et son menu : « Ouvrir le dossier de config », « Démarrer avec Windows » (case cochée/décochée), « Quitter »
- [ ] Démarrage avec Windows : actif après le premier lancement ; le moteur se relance à la prochaine ouverture de session
- [ ] Instance unique : lancer `nullpad.exe` une 2e fois ne crée pas de 2e icône
- [ ] Soundboard : un son s'entend dans le casque
- [ ] `SoundVolume` sur une molette baisse un son en cours (casque et VB-Cable) ; en mode `App`, la voix aussi
- [ ] Le son s'entend dans « CABLE Output » (enregistreur de sons Windows ou Discord)
- [ ] Rappui sur un son : `Restart` le relance, `Stop` l'arrête
- [ ] `StopSounds` coupe tous les sons
- [ ] Débrancher le casque pendant un son, puis le rebrancher
- [ ] Changer la sortie par défaut de Windows
- [ ] VB-Cable absent : l'icône propose de l'installer
- [ ] Mode `App` : voix audible dans « CABLE Output », molette droite = gain, clic = sourdine, latence ressentie
- [ ] Config rechargée à l'enregistrement ; une config cassée est ignorée (voir les logs) et l'ancienne reste active
