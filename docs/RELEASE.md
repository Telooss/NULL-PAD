# Publier une version de NULL/CONTROL

Les versions se construisent **en local** : GitHub Actions est bloqué pour ce compte (l'échec du contrôle CI du firmware sur les PR est normal).

## Prérequis

- tauri-cli 2 : `cargo install tauri-cli --version "^2" --locked`
- `gh` connecté (`gh auth login`)
- la clé de signature `%USERPROFILE%\.nullctl\updater.key` et son mot de passe

## La clé de signature

Générée une seule fois :

```powershell
cargo tauri signer generate -w "$env:USERPROFILE\.nullctl\updater.key"
```

La clé publique affichée est dans `ui/tauri.bundle.json`.

**Sauvegarder le fichier `.key` et son mot de passe en dehors du PC.** Si la clé est perdue, plus aucune mise à jour ne peut arriver aux installations existantes : il faut générer une nouvelle clé, mettre sa clé publique dans `ui/tauri.bundle.json`, puis réinstaller à la main chaque poste.

Le mot de passe peut être mis dans `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`, sinon le script le demande.

## Publier

```powershell
powershell -File scripts\release.ps1 -Version X.Y.Z -Notes "..."             # prépare
powershell -File scripts\release.ps1 -Version X.Y.Z -Notes "..." -Publish    # prépare, pousse et publie
```

Première version : `-Version 0.1.0`, permis tant qu'aucun tag `v0.1.0` n'existe.

## Ce que fait le script

1. Refuse si un contrôle échoue : format de version, version pas plus récente, arbre git sale, pas sur `main`, clé absente, clé publique vide, `gh` non connecté (seulement avec `-Publish`), tag déjà existant.
2. Lance les tests, clippy et node.
3. Change la version dans `tauri.conf.json`, `ui/Cargo.toml` et `engine/Cargo.toml`, puis `cargo update -w`.
4. Construit le moteur, le copie, puis lance `cargo tauri build --config tauri.bundle.json` avec l'environnement de signature (pour cette étape seulement).
5. Écrit `latest.json` et `notes.txt` dans `target\release\bundle\nsis\`.
6. Fait le commit `release: vX.Y.Z` et crée un tag annoté.
7. Avec `-Publish` : push, push du tag, puis `gh release create` avec le `setup.exe`, le `.sig` et `latest.json`. Sans `-Publish`, il affiche ces commandes.

Si une étape échoue après le changement de version, le script affiche « Pour reessayer : git checkout -- . ».

## Où est servie la mise à jour

`https://github.com/Telooss/NULL-PAD/releases/latest/download/latest.json`
