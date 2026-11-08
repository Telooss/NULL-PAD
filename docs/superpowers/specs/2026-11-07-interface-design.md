# Interface nullctl — conception

Date : 2026-11-07. Statut : **à valider par Alan**. Elle complète la conception générale `2026-11-05-logiciel-macropad-design.md` (§3.2, §7) ; en cas de conflit, ce document gagne pour l'interface.

## 1. Intention

Avec NULL/CONTROL (**nullctl**), le logiciel du pad **NULL/PAD**, Alan voit ce que fait chaque touche et la règle sans jamais toucher au JSON.

- **Vue** : le pad en 3D au centre, chaque touche porte sa fonction, et un vrai appui s'allume sur le rendu.
- **Réglage** : on glisse un fichier sur une touche, ou on clique pour un réglage fin.

Le tout avec l'identité « terminal hacktiviste × Y2K, sans kitsch », déjà fixée dans le design system Claude Design « nullctl ».

## 2. Objectifs, non-objectifs, anti-objectifs

**Objectifs**
- Un mode vue, sûr et en lecture seule.
- Un mode configuration à brouillon, profil par profil.
- Le glisser-déposer de fichiers sur une touche.
- Le réglage fin de toutes les actions du moteur (raccourci, texte, macro, lancer, ouvrir, commande, média, volume, micro, bascule, son, OBS, MIDI).
- Les réglages globaux.
- La gestion des profils (créer, renommer, supprimer).

**Non-objectifs (cette version)**
- Écrire dans le pad : touches natives, tags, molettes via ZMK Studio RPC. C'est le plan suivant. L'appli ne modifie que la config du moteur.
- Le changement automatique de profil selon l'appli au premier plan (spec §4).
- La note MIDI tenue, macOS et Linux.
- Le filet « rendu 3D simplifié » : prévu plus tard, si le besoin se présente (décision d'Alan).

**Anti-objectifs (échec même si ça marche)**
- Une modification qui change le comportement du pad avant d'être enregistrée.
- Une interface kitsch, ou confuse une fois tous les effets coupés.
- Une appli qui fait chauffer le GPU au repos, ou qui ajoute de la latence au pad.

## 3. Noms (décidés par Alan le 2026-11-07)

- **NULL/PAD** désigne le pad (boîtier, firmware, dépôt `Telooss/NULL-PAD`).
- **NULL/CONTROL**, en court `nullctl`, désigne le logiciel.

Le moteur, que j'avais mal nommé le 2026-11-05, est renommé :
- l'exécutable `nullpad.exe` devient `nullctl-engine.exe` ;
- l'interface est `nullctl.exe` ;
- le dossier de données `%APPDATA%\NULL-PAD` devient `%APPDATA%\nullctl` ;
- l'infobulle de l'icône devient « NULL/CONTROL » ;
- le mutex `Local\nullpad-engine` devient `Local\nullctl-engine`.

Au premier lancement, l'ancien dossier est migré : s'il existe et que le nouveau n'existe pas, il est renommé. La spec §10 est corrigée en conséquence.

## 4. Direction artistique : le design system « nullctl »

Source de vérité : https://claude.ai/artifact/JpPjDZvwtqksmzRf54H5Cw. On y lit `project/README.md` et `project/tokens.json`, ainsi que les composants `EcranPrincipal`, `PanneauTouche`, `CreationProfil` (README et preview), `bundle.css` / `bundle.js`, et le modèle `assets/Modele/nullpad.json`. On l'applique tel quel :

- **Structure** : barre de titre (36 px), barre des profils (76 px), scène 3D au centre, panneau de réglage de 400 px à droite. Un seul panneau ouvert à la fois, relié à la touche sélectionnée par un fil coudé `violet-400`.
- **Couleurs** : violet seul (`violet-500` pour les pleins, `violet-400` sur fond sombre). Fonds `bg-000`, `bg-100`, `surface-100`, `surface-200`.
- **Polices** :
  - IBM Plex Sans et Mono pour l'interface ;
  - Bagel Fat One pour les **noms de profil** uniquement ;
  - Silkscreen pour l'écran nice!view du rendu seulement.
- **Décor « terminal »** : uniquement dans la scène.
- **Glitch de 0,36 s** : seulement sur un changement de profil, une sauvegarde ou une création de profil. Il est coupé si Windows demande moins d'animations.
- **Actions** : une icône Lucide **et** un libellé. Les déclencheurs sont des onglets segmentés.
- **Le test** : tous les effets coupés, l'appli reste claire.

Les écrans qui n'ont pas de maquette (Réglages, mode configuration, glisser-déposer, mode vue en direct) suivent les mêmes composants et les mêmes tokens. Ils ne reçoivent **aucun** nouvel effet.

## 5. Architecture

- **Deux processus.**
  - Le **moteur** (`nullctl-engine.exe`, l'existant) tourne toujours en arrière-plan.
  - L'**interface** (`nullctl.exe`) est une appli **Tauri 2** (WebView2) qui se ferme sans conséquence pour le pad.
- **Config** : l'interface lit `config.json` et l'écrit **de façon atomique** (fichier temporaire puis renommage) quand on enregistre. Le moteur la recharge à chaud, comme aujourd'hui. Une écriture refusée par la validation du moteur est signalée dans l'interface, et le brouillon reste.
- **Événements en direct** : un named pipe Windows `\\.\pipe\nullctl-events`, créé par le moteur.
  - Le moteur y écrit une ligne JSON par événement : `key` (entrée, appui ou relâchement, profil), `action` (entrée, déclencheur, type d'action, `ok` ou une erreur), `profile` (profil actif), `link` (USB, BLE ou déconnecté), `config` (rechargée, ou rejetée avec la raison).
  - Plusieurs clients peuvent s'y brancher à la fois.
  - Sans client, le moteur n'écrit rien, et un client lent n'est jamais attendu : ses messages sont jetés.
  - Cet ajout au moteur fait partie de ce plan.
- **Validation partagée** : l'interface valide le brouillon avant de l'enregistrer avec les **mêmes** règles que le moteur. Elle appelle la crate `engine` comme bibliothèque, depuis le côté Rust de Tauri, avec `config::parse`. Les règles ne sont pas dupliquées en JavaScript.
- **Interface web** : HTML/CSS/JS avec les tokens et `bundle.css` du design system. Le framework est au choix du builder, à condition de rester léger : le design system n'utilise pas React.

## 6. Le pad en 3D (règles de performance non négociables)

- La scène three.js est construite depuis `nullpad.json`. Focale de 16°, inclinaison de 24° (design system).
- **Rendu à la demande** : une image n'est dessinée que lorsque quelque chose change (survol, sélection, appui reçu du moteur, glitch, rotation, redimensionnement). Au repos, **aucune** boucle de rendu ne tourne. Fenêtre réduite ou cachée : rien n'est dessiné.
- **Densité de pixels** plafonnée à 1,5.
- **Légendes** des keycaps, nice!view, « NULL/PAD » : ce sont des textures, redessinées seulement quand leur contenu change.
- **Pièces jamais visibles** du modèle : on peut les retirer au chargement, si elles se trouvent derrière une coque opaque.
- **Sélection et survol** : lancer de rayon sur les keycaps et les molettes (design system).

## 7. Mode vue (par défaut)

- **Légendes** : chaque touche et chaque molette (sens horaire, anti-horaire, clic) affiche sa fonction dans le profil affiché.
  - La légende est **automatique** : nom du son, de la scène, de l'appli, raccourci (« CTRL+C »), « VOL + », chiffre pour une touche native…
  - Elle peut être **remplacée** par un texte à soi, nouveau champ `label` optionnel d'une liaison, ignoré par le moteur.
  - Elle tient sur 2 lignes courtes. Le builder fixe la longueur et la troncature.
- **Touches natives** (couche 0 du firmware, comme le pavé numérique) : le moteur ne voit pas leur code. L'interface les connaît par une table de la keymap par défaut du firmware, et les affiche (« 7 », « + », « ENTRÉE »). Elle les présente comme « native (firmware) », non modifiables dans cette version.
- **Appui en direct** :
  - un événement `key` enfonce et allume la touche du rendu ;
  - un événement `action` affiche une ligne courte (« K07 → Son : airhorn »), ou l'erreur, qui s'efface après quelques secondes ;
  - un événement `profile` change le profil affiché, avec le glitch.
- **Clic sur une touche** : le panneau s'ouvre en lecture seule, sans contrôle d'édition visible ; un bouton « Configurer » y mène.
- **Glisser-déposer** : refusé, avec le message « Passer en configuration pour modifier ».
- **Toujours visibles dans la barre des profils** :
  - le profil actif ;
  - la connexion (USB, BLE ou déconnecté, d'après le moteur) ;
  - l'état du moteur (en marche ou arrêté, selon que le pipe répond).

## 8. Mode configuration (brouillon par profil)

- **Entrée** : le bouton « Configurer » de la barre des profils. Il prend le profil affiché.
- **Signalement fort** : un bandeau « CONFIG // NON ENREGISTRÉ » dans le style terminal et un cadre de scène violet.
- **Brouillon** : toutes les modifications portent sur une copie du profil. Le moteur, donc le pad, n'en voit rien.
- **« Enregistrer »** :
  1. valide la config entière (brouillon fusionné) avec `config::parse` ;
  2. écrit le fichier de façon atomique ;
  3. attend l'événement `config` du moteur pour afficher « enregistré », ou l'erreur ;
  4. déclenche le glitch de sauvegarde et quitte le mode.
- **« Annuler »** : jette le brouillon, avec une confirmation s'il y a des changements.
- **Changer de profil affiché, ou quitter l'appli, avec un brouillon modifié** : la question « Enregistrer / Abandonner / Rester » s'affiche.
- **Profils** :
  - **créer** : écran mis en scène (`CreationProfil`) pour le nom et le tag de 3 caractères `[A-Z0-9]`, avec l'id libre suivant, de 1 à 8 ;
  - **renommer** : dans le brouillon du profil ;
  - **supprimer** : à part, avec une confirmation qui nomme le profil.
  - La création et la suppression s'enregistrent immédiatement, après confirmation.
- **Glisser-déposer sur une touche ou une molette** :
  - la cible visée s'allume en violet pendant le survol (lancer de rayon) ;
  - au lâcher, l'action est déduite :
    - `.mp3`, `.wav`, `.ogg` ou `.flac` → `Sound` (le son est ajouté à la bibliothèque, avec un id tiré du nom de fichier et rendu unique) ;
    - `.exe` ou `.lnk` → `Launch` ;
    - `.ps1` → `Launch` de `powershell` avec `-NoProfile -ExecutionPolicy Bypass -File <chemin>`, `.bat`/`.cmd` → `Launch` du fichier lui-même : arguments en liste (argv), jamais une chaîne de shell, pour qu'un nom de fichier ne puisse pas injecter de commande ;
    - URL → `Open` ;
    - autre fichier → `Open` ;
  - le panneau s'ouvre avec l'action préremplie, pour vérification ;
  - plusieurs fichiers lâchés d'un coup : seul le premier compte, et un message le signale.
- **Panneau de réglage (une touche ou une molette)** :
  - action en cours en tête, puis « Changer d'action » (icône et libellé pour chacune) ;
  - déclencheurs en onglets (appui, appui long, double ; bascule = action `Toggle`) ;
  - champs propres à l'action :
    - sélecteur de fichier Windows pour un programme, un fichier ou un son ;
    - arguments ;
    - capture de raccourci ;
    - éditeur de macro (étapes et attentes) ;
    - choix de la scène ou de la source OBS (liste lue dans OBS s'il est joignable, sinon saisie libre) ;
    - canal, note et CC MIDI ;
  - légende personnalisée.

## 9. Écran Réglages (enregistré à part)

On l'ouvre par le bouton de droite de la barre des profils. Il a son propre brouillon, avec « Enregistrer » et « Annuler ».

- **Audio** :
  - sortie casque ;
  - VB-Cable : détecté ou absent, avec le lien `https://vb-audio.com/Cable/` ;
  - volumes de la soundboard, du casque et de VB-Cable ;
  - mode micro « Externe » ou « Via l'appli », avec l'avertissement sur le délai et le micro muet sans moteur.
- **Sons** : la bibliothèque. Ajouter (sélecteur de fichier ou glisser-déposer), retirer, régler le volume, le rappui (relancer ou couper) et le nom.
- **OBS** : hôte (avec le conseil de garder `127.0.0.1`), port, mot de passe masqué, et « Tester la connexion ».
- **MIDI** : le nom du port, et « Tester », qui envoie un CC.
- **Démarrage** : lancer le moteur avec Windows, temps de l'appui long et du double appui.

Les boutons « Tester » passent par le moteur, à travers une requête sur un second pipe, `\\.\pipe\nullctl-control`. Ils ne dupliquent pas les clients OBS et MIDI dans l'interface. Le builder peut fusionner les deux pipes en un seul pipe bidirectionnel s'il garde les mêmes garanties.

## 10. Contraintes

- Windows 11, Tauri 2, WebView2.
- L'interface ne bloque jamais le moteur, et le moteur n'attend jamais l'interface.
- Aucune information n'est seulement dans un effet. Contraste et accessibilité suivent les tokens du design system.
- Les configs existantes restent valides : le champ `label` est optionnel, et `version` reste 1.

## 11. Laissé au builder

Il décide :
- le framework web léger ;
- la forme exacte des messages du pipe, dans les limites du §5 ;
- la longueur et la troncature des légendes ;
- le découpage du code de l'interface ;
- la façon de retirer les pièces invisibles du modèle 3D.

## 12. Points tranchés après la relecture du builder (choix de Claude)

1. **Design system dans le dépôt.**
   - Les fichiers `project/` de l'artefact « nullctl » sont copiés dans `ui/design/`, tels quels, comme base de départ : tokens, `bundle.css`, `bundle.js`, previews, `assets/Modele/nullpad.json`.
   - `bundle.js` fournit déjà `NullCtl.pad3d` (scène, légendes en textures, lancer de rayon, sélection), `shell`, `frame`, `wire` et `glitch`. On le reprend et on l'adapte au rendu à la demande (§6), sans réécrire la 3D.
   - Correspondance entre les pièces et les entrées :
     - `F1`–`F20` = `K00`–`K19` (rang par rang) ;
     - `F21`–`F23` = M1–M3 ;
     - `D1` / `D2` = molettes gauche et droite (sens et clic).
   - Le README de `PanneauTouche` dit « tout s'enregistre au fil de l'eau ». Ce point est **remplacé** par le mode configuration à brouillon (§8, décision d'Alan du 2026-11-07).
2. **Cycle de vie.**
   - L'icône du moteur gagne « Ouvrir nullctl », qui lance l'interface ou ramène sa fenêtre devant.
   - L'interface est en instance unique, et la fermer la quitte vraiment (pas de réduction dans la zone de notification).
   - Si le moteur est arrêté, la barre l'affiche, avec un bouton « Démarrer le moteur » qui lance `nullctl-engine.exe`.
   - L'entrée de démarrage avec Windows est migrée comme le dossier de données : l'ancienne valeur est supprimée et la nouvelle pointe vers `nullctl-engine.exe`.
   - Les deux exécutables vivent côte à côte dans le même dossier. L'installeur n'est pas prévu dans cette version.
3. **« Enregistrer » sans moteur** : le fichier est écrit quand même, et l'interface affiche « Enregistré, pris en compte au démarrage du moteur ».
   - Si le moteur rejette un fichier que l'interface avait accepté (versions différentes), il garde la dernière config valide. L'interface affiche alors l'erreur reçue par le pipe et propose de rouvrir le brouillon, qu'elle garde en mémoire jusqu'à la confirmation.
4. **Profil changé sur le pad pendant une configuration** : l'affichage reste sur le profil en cours de modification. Une notification indique « Le pad est passé sur STR ». On suit à nouveau le pad en sortant du mode configuration.
5. **Touches natives.**
   - La table de leurs légendes est **générée** au build depuis `firmware/null_pad.keymap` (couche 0, codes `KP_*`). Elle n'est pas écrite à la main.
   - Une touche native n'accepte pas de liaison du moteur dans cette version. Le dépôt d'un fichier dessus est refusé avec « Touche native du firmware : modifiable plus tard depuis le pad ». La validation de l'interface refuse le cas.
6. **Glisser-déposer** : fichiers seulement (le gestionnaire natif de Tauri, qui donne la position pour le lancer de rayon). Une URL se saisit au clavier dans le panneau, avec l'action « Lancer ».
