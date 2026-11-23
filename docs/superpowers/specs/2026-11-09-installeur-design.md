# Installeur et mise à jour automatique : conception

Date : 2026-11-09. Validé par Alan dans la conversation : « propose ou non de garder les fichiers quand on désinstalle, le reste me paraît bon ; le firmware de côté, mais en axe d'amélioration prévu ».

## 1. Intention

On peut donner NULL/CONTROL à quelqu'un sans qu'il touche à GitHub ni à Rust. Il lance un installeur une fois. Ensuite, le logiciel se met à jour depuis l'appli, en un clic.

- **Réussite** : une personne qui n'a jamais vu le dépôt installe nullctl, et reçoit plus tard la version suivante depuis l'appli, sans rien retélécharger à la main.
- **Anti-objectif** : une mise à jour qui coupe le moteur sans prévenir (en plein jeu ou en plein live). Une mise à jour qui installe un fichier non signé par Alan. Une désinstallation qui efface la config sans l'avoir demandé.

## 2. Ce qui est décidé

### Installeur
- Installeur Windows NSIS, en français, `NULL-CONTROL_<version>_x64-setup.exe`.
- Installation **pour l'utilisateur courant**, dans `%LOCALAPPDATA%\NULL-CONTROL` (dossier imposé par Tauri). Pas de droits administrateur, ni pour l'installation ni pour les mises à jour.
- Un raccourci dans le menu Démarrer.
- Il contient `nullctl.exe` (l'interface) et `nullctl-engine.exe` (le moteur), dans le même dossier. C'est déjà là que l'interface cherche le moteur (`settings::engine_exe`).
- **Désinstallation** :
  - arrête le moteur ;
  - retire le démarrage automatique (valeur `nullctl` de `HKCU\…\Run`) ;
  - **propose** de supprimer la config et les données (`%APPDATA%\nullctl\`) avec une case à cocher, décochée par défaut : on garde.

### Mise à jour
- nullctl vérifie au démarrage, puis toutes les 6 h.
- Quand une version plus récente existe, un message « Mise à jour X disponible » s'affiche avec ses notes et un bouton **Installer**. Il n'y a pas d'installation silencieuse.
- Au clic, dans l'ordre :
  1. téléchargement, avec la progression affichée ;
  2. vérification de la signature ;
  3. arrêt propre du moteur ;
  4. installation ;
  5. relance de nullctl ;
  6. relance du moteur s'il tournait avant.
- Réglage « Vérifier les mises à jour », activé par défaut.
- Les fichiers sont hébergés dans les Releases du dépôt public `Telooss/NULL-PAD`. L'appli lit `https://github.com/Telooss/NULL-PAD/releases/latest/download/latest.json`. L'utilisateur ne voit jamais GitHub.
- **Signature** : les mises à jour sont signées avec la clé de mise à jour Tauri (minisign) d'Alan. nullctl refuse un fichier dont la signature ne correspond pas à la clé publique intégrée.

### Publication (côté Alan)
- Clé de signature générée une fois par Alan, avec un mot de passe. Elle est rangée **hors du dépôt**, dans `%USERPROFILE%\.nullctl\updater.key`.
  - À sauvegarder ailleurs : si elle est perdue, plus aucune mise à jour n'est possible pour les installations existantes.
  - La clé publique va dans la config Tauri.
- Une commande par version : `scripts\release.ps1 -Version X.Y.Z`. Elle :
  - vérifie que l'arbre est propre ;
  - monte les numéros de version ;
  - lance les tests ;
  - compile le moteur et l'interface ;
  - construit et signe l'installeur ;
  - écrit `latest.json` ;
  - crée le commit et le tag.
- La publication (push et release GitHub) se fait avec `-Publish`, lancé par Alan.
- Les builds restent **locaux** : GitHub Actions est bloqué par la facturation du compte.

### Moteur
- Nouvelle requête de contrôle `quit` sur `\\.\pipe\nullctl-control` : le moteur s'arrête proprement, comme « Quitter » dans la zone de notification.
- Nouvel argument `nullctl-engine.exe --quit` : il envoie `quit` au moteur en marche et attend sa sortie, avec un délai. L'installeur s'en sert, car il ne parle pas au pipe lui-même.

## 3. Hors sujet, prévu plus tard

- **Mise à jour du firmware** (axe d'amélioration prévu) : nullctl signalerait un nouveau `.uf2` et le copierait sur le pad en mode bootloader.
- Certificat de signature de code Windows (payant). Sans lui, la **première** installation affiche l'avertissement SmartScreen (« Exécuter quand même »). Les mises à jour suivantes passent par nullctl.
- macOS et Linux.

## 4. Laissé au constructeur

- Le mécanisme Tauri exact pour embarquer le moteur (`externalBin` ou `resources`). Une contrainte : `cargo build` et `cargo test` doivent continuer à marcher sans installeur ni moteur compilé, donc la config d'empaquetage passe par un fichier séparé donné à `cargo tauri build --config`.
- Le nom des fichiers. `productName` contient « / » (« NULL/CONTROL »), ce qui est interdit dans un chemin. L'installeur utilise donc « NULL-CONTROL », tandis que le titre de la fenêtre et les textes gardent « NULL/CONTROL ». Les exe gardent leurs noms actuels (`nullctl.exe`, `nullctl-engine.exe`).
- Où ranger la préférence « Vérifier les mises à jour ». `localStorage` suffit, car il est persistant dans la webview Tauri.
- La forme du message de mise à jour, en réutilisant le design system existant (toast ou bandeau).
