# Logiciel de NULL/PAD — conception

Date : 2026-11-05. Statut : validé par Alan le 2026-11-05. Noms choisis le 2026-11-05, révisés le 2026-11-07 : **NULL/PAD** pour le pad, **NULL/CONTROL (nullctl)** pour le logiciel (§10).

## 1. Intention

Alan a un clavier 60 %, sans pavé numérique. Le macropad (23 touches, 2 molettes cliquables, écran nice!view, firmware ZMK, voir `PROJECT.md` du dossier matériel, hors de ce dépôt) doit faire trois métiers :

1. **pavé numérique** au quotidien ;
2. **soundboard et streaming** : sons dans le micro en jeu (aujourd'hui Soundpad), OBS ;
3. **Ableton et montage vidéo** : raccourcis et réglages.

Il veut **son propre logiciel**, pas ZMK Studio : toutes les fonctions d'un vrai macropad (type Stream Deck / Logitech G HUB), les molettes comprises, avec **la latence la plus basse possible** entre l'appui et l'action, et une identité visuelle à lui.

## 2. Objectifs, non-objectifs, anti-objectifs

**Objectifs**
- Assigner n'importe quelle action à n'importe quelle touche ou molette, par profil.
- Profils illimités, nommés librement.
- Soundboard intégrée qui joue dans le micro (remplace Soundpad).
- Latence logicielle négligeable (< 1 ms entre la réception du code et le lancement de l'action).
- Une interface à l'identité forte mais **claire** : l'utilisateur ne doit jamais être perdu.

**Non-objectifs (v1)**
- macOS et Linux : Windows 11 uniquement.
- Plusieurs macropads en même temps, synchro cloud, boutique de profils.
- Écrire notre propre pilote audio (on s'appuie sur VB-Cable).

**Anti-objectifs (échec même si ça marche)**
- Le pavé numérique ne marche plus quand le logiciel est fermé.
- Une interface kitsch (Hello Kitty, cœurs, rose partout, étoiles partout) ou confuse.
- Une latence perceptible, en jeu ou dans Ableton.

## 3. Architecture (décidée)

Deux morceaux :

### 3.1 Firmware (ZMK, sur le macropad)
- **Un profil = une couche ZMK.** Le profil actif vit donc dans le macropad lui-même : il marche sans logiciel et l'écran le connaît.
- **Par défaut, les 3 touches du bas (M1, M2, M3) sélectionnent les profils 1, 2, 3** (exigence d'Alan, ce sont les touches visibles sur le modèle 3D). L'utilisateur peut déplacer ces touches de profil.
- **L'écran nice!view affiche le profil actif** : son numéro, ou un **tag de 3 lettres** choisi par l'utilisateur (nom de couche ZMK).
- Dans un profil, chaque touche est soit **native** (une vraie touche clavier envoyée par le firmware, ex. les chiffres du pavé : marche partout, sans logiciel), soit **action logiciel** : elle envoie un **code unique** que personne n'utilise (F13–F24 combinés avec Ctrl/Maj/Alt), que le logiciel capte. Idem pour chaque sens et chaque clic de molette.
- **Révision au plan moteur (2026-11-05)** : les codes uniques ne sont plus F13–F24 + modificateurs (le modificateur fuirait vers les jeux : Ctrl fantôme), mais des **usages HID Consumer réservés** lus en Raw Input, qui encodent profil + touche. Détails et repli dans `docs/superpowers/plans/2026-11-05-moteur.md`. Le paragraphe suivant reste valable pour l'annonce de profil à l'interface.
- Le logiciel doit savoir quel profil est actif. Choix de Claude : chaque touche de profil fait `&to N` **et** envoie un code « profil N » ; les codes des touches peuvent alors être les mêmes dans tous les profils. Le builder peut préférer une autre méthode si elle est fiable.
- Molette gauche par défaut = volume (touches média natives), clic = mute. Molette droite = action logiciel (le micro a besoin du logiciel en mode « via l'appli », voir §6 ; en mode « externe », volume micro Windows, aussi via le logiciel).
- **Choix de Claude** : ZMK Studio reste activé dans le firmware (`CONFIG_ZMK_STUDIO=y`, `&studio_unlock` dans la keymap). Notre logiciel parle le protocole RPC de ZMK Studio (protobuf, ouvert) en USB pour écrire dans le macropad, sans reflasher : touches natives, place des touches de profil, tag de 3 lettres, et attribution des codes uniques. Repli si ce client RPC s'avère trop lourd : renvoyer vers l'appli officielle ZMK Studio pour ces réglages.
- Nombre de profils borné par les couches réservées dans le firmware (ex. 8, au choix du builder selon la mémoire).
- Anti-rebond ZMK raccourci par rapport aux 5 ms par défaut, valeur à régler sur le vrai PCB.
- Build par GitHub Actions (déjà décidé dans `PROJECT.md`).
- **Révision au plan firmware (2026-11-06)** : les molettes passent par des touches cachées (contournement détaillé dans `docs/superpowers/plans/2026-11-06-firmware.md`) ; le build se fait en local sous WSL (`firmware/build-local.sh`), les Actions restant possibles.

### 3.2 Logiciel PC (Windows)
- **Moteur en Rust**, natif, en arrière-plan (icône dans la barre des tâches, démarre avec Windows). Hook clavier bas niveau : il intercepte les codes uniques, **les bloque** pour qu'aucune autre appli ne les voie, et exécute l'action du profil actif. Aucune attente sur l'interface.
- **Interface en Tauri**, process ou fenêtre séparée, qui peut être fermée pendant l'usage. Elle lit et écrit la config, le moteur la recharge à chaud.
- Config en fichiers locaux lisibles (JSON ou TOML, au choix du builder), import/export de profils.

### 3.3 Latence
- Le gros de la latence est le lien clavier → PC : USB ~1–5 ms, Bluetooth ~7–30 ms. Recommandation affichée dans l'appli : USB pour le jeu et Ableton.
- L'état de connexion (USB / BLE) est affiché en permanence dans l'interface (si détectable, sinon au choix du builder).

## 4. Profils

- Au premier lancement : un seul profil, **« Default »** (profil 1, sur M1). M2 et M3 pointent vers des emplacements vides jusqu'à ce qu'on crée les profils 2 et 3.
- L'utilisateur crée, renomme, duplique, supprime des profils, dans la limite des couches du firmware (§3.1). Chaque profil a un nom long (affiché dans l'appli) et un tag de 3 lettres (affiché sur l'écran du macropad).
- Le pavé numérique d'Alan sera simplement un profil dont les touches sont natives.
- La création d'un profil est un **moment mis en scène**, façon écran de création de profil de SSX Tricky (on choisit le nom comme on nomme son perso). Interprétation de Claude, à confirmer sur maquette.
- N'importe quelle touche peut devenir une touche « aller au profil X » ou « profil suivant / précédent ».
- **Changement automatique (à évaluer, dépend d'un canal PC → macropad)** : pour basculer la couche du macropad depuis le PC, il faut que ZMK Studio RPC le permette ou un module ZMK « raw HID » de la communauté. Sans ce canal, la fonction est abandonnée (pas de profil côté logiciel seulement, sinon l'écran mentirait).
- **Changement automatique de profil selon l'appli au premier plan** (ex. Ableton au premier plan → profil Studio). Règles (choix de Claude) : chaque profil a une liste d'applis (vide par défaut = jamais choisi automatiquement) ; quand une appli de la liste passe au premier plan, on bascule sur ce profil ; quand elle perd le premier plan, on revient au profil d'avant ; un changement manuel l'emporte jusqu'au prochain changement d'appli au premier plan. Un interrupteur global coupe la fonction.

## 5. Actions disponibles

Sur une touche (appui court par défaut) :
- raccourci clavier / combinaison, touche média ;
- taper un texte ;
- macro : suite d'actions avec délais ;
- lancer une appli, ouvrir un fichier ou une URL ;
- **exécuter une commande ou un script** (PowerShell, .bat, programme + arguments) — demandé explicitement ;
- **son de la soundboard** (voir §6) ;
- **OBS** via obs-websocket v5 : changer de scène, couper/rétablir une source, démarrer/arrêter stream ou enregistrement, sauvegarder le replay ;
- volume Windows, mute micro ;
- aller à un profil ;
- **MIDI** (note, CC) vers un port MIDI virtuel, pour Ableton. Le moyen de créer le port (Windows MIDI Services, loopMIDI…) est au choix du builder.

Déclencheurs par touche : appui court, appui long, double appui, et action « bascule » (on/off). Le double appui ne retarde que les touches qui l'utilisent.

Sur une molette : une action par sens de rotation + une au clic. Pas réglable (ex. ±2 % par cran).
Défauts : **gauche = volume général**, **droite = niveau du micro**, clic = mute (couper/rétablir).

## 6. Soundboard

- Sons chargés en mémoire (mp3, wav, ogg) : aucun délai de chargement à l'appui.
- Sortie double : casque (pour s'entendre) + **micro virtuel VB-Cable** (le jeu, Discord ou OBS le choisissent comme micro).
- **Le trajet du micro est un réglage** (décidé avec Alan : la personnalisation est le but) :
  - **Mode « externe » (défaut)** : le vrai micro rejoint VB-Cable sans le logiciel (Windows « Écouter ce périphérique », ou l'interface audio de l'utilisateur ; Alan gère son entrée via sa Focusrite). Le logiciel n'ajoute que les sons. Le micro marche même logiciel fermé. La molette droite règle le volume Windows du micro.
  - **Mode « via l'appli »** : le logiciel capte le micro et le mélange aux sons dans VB-Cable. Contrôle total du mélange ; la molette droite règle ce gain. Coût : ~10–30 ms en WASAPI partagé (le mode basse latence est un plus, pas une promesse), et micro muet si le logiciel est fermé — l'interface le dit clairement quand on choisit ce mode.
- Audio WASAPI ; latence de lecture des sons la plus basse possible.
- Options : volume par son, « stop tous les sons », comportement au rappui (relancer ou couper).
- **Volume de la soundboard** (Alan, 2026-11-05) : un réglage de tout ce que la soundboard sort, assignable à une molette ou une touche ; en mode « via l'appli », il règle aussi le micro mélangé. Alan reste en mode « externe » : pas de délai sur sa voix.
- VB-Cable s'installe à part (gratuit). Si absent, l'appli le détecte et explique comment l'installer.

## 7. Direction artistique (décidée avec Alan)

**Structure, comme Logitech G HUB** : le macropad au centre, grand, en **rendu 3D réaliste tiré du vrai modèle** (`case/out/macropad-assemblage.step`, pièces en STL dans `case/out/`). On clique sur une touche ou une molette du rendu pour la régler. Profils en haut. Réglages globaux (audio, VB-Cable, OBS, démarrage) sur un écran à part.

**Identité : DedSec (néo-punk, hacktiviste) × Y2K (années 2000), sans kitsch.**
Références fournies par Alan : bureau collage de fenêtres Win98/XP/Mac OS X, site Flash *Godzilla The Series* (1999), affiche Y2K Notepad, typo « SymbolScape », police « Digistick », affiche « Y2K design » sur papier noir. Ce qu'on en garde :
- fond **noir légèrement texturé** (pont avec DedSec) ;
- couleurs : le **violet** du macropad, seul (décision d'Alan du 2026-11-05 : pas d'accent vert phosphore) ;
- le macropad posé dans un **cadre d'appareil** façon Flash 1999 (bord arrondi, aérations, voyants) ;
- **typo bombée / autocollant** réservée aux noms de profil ;
- détails tech : trame de points, codes-barres, petits textes de registres, étoiles à 4 branches (une visible à la fois maximum) ;
- **glitch** (décalage RGB, lignes de balayage) seulement sur les événements : changement de profil, sauvegarde ; une fraction de seconde, il ne cache jamais une information ;
- collage de fenêtres de vieux systèmes en **décor** (accueil, création de profil), jamais là où l'on règle.

**Direction retenue (2026-11-05, tour Nano Banana)** : « terminal hacktiviste » (prompt C). Cadre fin noir mat aux coins en équerre, trame de points en fondu sur les bords, textes de registre monospace verticaux (« 0x4D50 // NODE 01 »), code-barres, une étoile à 4 branches, scotch et fenêtres Win98 scotchées qui racontent quelque chose (« SYSTEM.INI — chargement du profil… »). Écartés : châssis graphite (« trop clean / lisse »), coque violette fumée.

**Règles de clarté (non négociables)**
- Couche usage toujours lisible : police simple, bon contraste, chaque chose toujours au même endroit (profils en haut, macropad au centre, réglages à droite).
- Un seul panneau de réglage ouvert à la fois, au même endroit.
- La touche sélectionnée et son panneau sont reliés visuellement.
- Toujours visibles : profil actif, connexion USB/BLE, et si les changements de config sont enregistrés (pas l'état d'enregistrement OBS).
- Menus d'actions : icône **et** texte.
- Test : sans aucun effet, l'appli reste parfaitement utilisable.

**Ce qui a été refusé** (ne pas y revenir) : interfaces néon-graffiti, chrome clair, riso/stickers en plein écran (« kitsch ») ; interface au trait façon croquis technique (« moche »).

**Méthode** : Alan juge sur maquettes. Claude écrit les prompts, **Alan génère les images avec Nano Banana** et les renvoie (pas d'accès API) ; puis maquettes HTML. Chaque tour de DA attend donc le retour d'Alan. Toujours vérifier une capture avant de montrer.

## 8. Contraintes

- Windows 11, un seul PC.
- Matériel figé par `PROJECT.md` : nRF52840 (SuperMini), ZMK, nice!view, 2 × PEC11R, 23 touches 1u.
- Le firmware ZMK ne parle pas MIDI. Le seul canal PC → macropad standard est ZMK Studio RPC (édition de la keymap) ; les actions passent par les codes uniques.

## 9. Laissé au builder

- Format de config, structure du code, choix des crates Rust.
- Mécanisme exact de hook (WH_KEYBOARD_LL ou Raw Input) tant que les codes sont bloqués et la latence tenue.
- Comment produire le rendu 3D (conversion STEP → glTF, three.js dans Tauri…).
- Port MIDI virtuel.
- Vérifier que le widget nice!view affiche bien le nom de couche (sinon petit widget ZMK maison pour le numéro / tag de 3 lettres).

## 10. Ouvert

- **Risque non vérifié** : certains jeux à anti-triche ignorent les frappes simulées ou tournent en administrateur. Les actions « raccourci » pourraient ne pas marcher en jeu ; les sons dans le micro ne sont pas concernés.

- **Nom (décidé par Alan le 2026-11-05)** : le macropad s'appelle **NULL/PAD** (capitales, avec le slash : rendu du pad, en-tête, barre de titre ; dépôt `Telooss/NULL-PAD`). Révisé par Alan le 2026-11-07 : le logiciel s'appelle **NULL/CONTROL**, ou **nullctl** là où le slash est interdit (moteur `nullctl-engine.exe`, interface `nullctl.exe`, config `%APPDATA%\nullctl\`, migrée depuis `%APPDATA%\NULL-PAD\`). La piste nullctl, écartée le 2026-11-05, est donc retenue. L'identité passe par la typo du nom, jamais par un symbole emprunté (pas de crâne, rien qui imite DedSec / Watch Dogs).