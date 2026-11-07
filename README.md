# NULL/PAD

Logiciel Windows du **NULL/PAD**, un macropad fait maison : 23 touches, 2 molettes cliquables, écran nice!view, firmware ZMK, en USB ou Bluetooth.

Le but : tout ce que fait un Stream Deck ou Logitech G HUB, molettes comprises, avec **la latence la plus basse possible** entre l'appui et l'action. Le tout sans dépendre du logiciel pour les touches de base : le pavé numérique marche même logiciel fermé.

## Le macropad

| | |
|---|---|
| Touches | 20 en grille 5 × 4 + 3 en bas (M1, M2, M3 : choix du profil par défaut) |
| Molettes | 2 × PEC11R cliquables (gauche : volume, droite : micro, configurables) |
| Écran | nice!view, affiche le profil actif (numéro ou tag de 3 lettres) |
| Cerveau | nRF52840 (SuperMini), firmware ZMK |

## Ce que fait le logiciel

Un **profil** = une couche du firmware. Dans chaque profil, une touche est soit **native** (vraie touche clavier, marche partout), soit **action** : elle envoie un code HID réservé que le moteur capte et transforme en action.

| Morceau | État |
|---|---|
| **Moteur** : raccourcis, texte, macros, lancer un programme, ouvrir un lien, commande PowerShell, touches média, volume, micro. Appui, appui long, double appui, bascule. Config rechargée à chaud. | ✅ fait, à valider sur le vrai pad |
| Soundboard intégrée qui joue dans le micro (remplace Soundpad) : sons mp3 / wav / ogg / flac vers le casque et VB-Cable, volume général, micro optionnel. | ✅ fait, à valider sur le vrai pad |
| OBS (websocket) et Ableton (MIDI virtuel) | ✅ fait, à valider sur le vrai pad |
| Firmware ZMK (profils, codes action, tag sur l'écran) | ✅ compile (local, WSL), à valider sur le vrai pad |
| Écriture dans le pad depuis l'appli (ZMK Studio RPC) | à venir |
| Interface (rendu 3D du pad, esthétique terminal hacktiviste Y2K, violet) | à venir |

## Compiler et lancer

Prérequis : Windows 11, [Rust](https://rustup.rs) stable.

```powershell
cargo build --release          # produit target\release\nullctl-engine.exe
cargo run -p engine --release  # lance le moteur (icône dans la zone de notification)
cargo test
```

Sans le pad, `--simulate` remplace le macropad par des lignes tapées au clavier. Détails, format de la config et liste des actions : [engine/README.md](engine/README.md).

Config et journaux : `%APPDATA%\nullctl\`.

## Organisation

```
engine/                         moteur Rust (arrière-plan, Raw Input, actions)
firmware/                       firmware ZMK du pad (config, shield, build local, README)
docs/superpowers/specs/         conception du logiciel (intention, architecture, direction artistique)
docs/superpowers/plans/         plans d'implémentation et suivi
```

## Documentation

- [Conception du logiciel](docs/superpowers/specs/2026-11-05-logiciel-macropad-design.md)
- [Plan du moteur](docs/superpowers/plans/2026-11-05-moteur.md) et [points de suivi](docs/superpowers/plans/2026-11-05-moteur-suivi.md)
- [Moteur : usage, config, codes HID](engine/README.md)
