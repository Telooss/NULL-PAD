# Guide NULL/CONTROL : configurer chaque touche

Ce guide explique comment donner un rôle à chaque touche et à chaque molette du NULL/PAD avec **nullctl** : raccourcis, scènes OBS, notes MIDI, lancement d'applis, sons, etc.

## 1. Avant de commencer

1. **Installe nullctl** : `NULL-CONTROL_<version>_x64-setup.exe`, sur la [page des versions](https://github.com/Telooss/NULL-PAD/releases/latest). Si SmartScreen avertit, clique sur « Informations complémentaires », puis sur « Exécuter quand même ».
2. **Le moteur doit tourner**, c'est lui qui exécute les actions. Il a une icône dans la zone de notification, et l'interface affiche « Moteur en marche ». S'il est arrêté, le bouton « Démarrer le moteur » le relance.
3. **Branche le pad en USB** pour modifier les touches natives (voir plus bas). Les touches « action » marchent aussi en Bluetooth. En Bluetooth, la pastille indique « Lecture seule en Bluetooth » et, dans « Configurer », les touches qu'on ne peut pas modifier sont grisées sur le pad 3D.
4. **Ferme ZMK Studio** : un seul programme à la fois peut parler au pad.

## 2. Les deux sortes de touches

Chaque touche, dans chaque profil, est de l'une de ces deux sortes :

| Sorte | Ce qu'elle fait | Marche sans nullctl ? | Exemples |
|---|---|---|---|
| **Native** | Le pad envoie une vraie touche de clavier. | Oui, partout, même logiciel fermé | chiffres du pavé, Entrée, une lettre, Volume +, Ctrl+C |
| **Action** | Le pad envoie un code réservé, que le moteur transforme en action. | Non, le moteur doit tourner | scène OBS, lancer Discord, jouer un son, note MIDI, macro |

**Règle simple :** pour une seule touche ou une combinaison fixe que tu veux partout (même sur un autre PC), prends **native**. Pour tout le reste, prends **action**.

## 3. Configurer une touche, pas à pas

1. Ouvre nullctl et choisis le profil en haut de la fenêtre (DEF, P02, etc.).
2. Clique sur **Configurer**.
3. Si l'appli affiche « Appuie sur M1 + M3 pour déverrouiller le pad », appuie sur **M1 et M3 en même temps** sur le pad. La pastille passe à « Pad modifiable ».
4. Clique sur la touche, sur le pad 3D. Le panneau de la touche s'ouvre.
5. Choisis ce qu'elle doit faire :
   - **Touche native** : clique sur « Changer », puis choisis la touche dans la liste. Tu peux aussi cliquer sur « Capturer » et appuyer sur la touche sur ton clavier. Les cases Ctrl / Maj / Alt / Win ajoutent des modificateurs, par exemple Ctrl + C.
   - **Action** : si la touche est native, clique d'abord sur « Rendre à nullctl (action) ». Choisis ensuite le type d'action dans la liste (section 5).
6. Facultatif : remplis **Légende personnalisée** pour changer le texte affiché sur la touche dans l'appli.
7. Clique sur **Enregistrer**. L'appli écrit d'abord le pad, puis la config. Si le pad refuse, rien n'est perdu : le brouillon reste.

**Annuler** abandonne tous les changements depuis « Configurer ».

> **Clavier AZERTY** : la liste des touches natives affiche ce que la touche tape avec *ta* disposition Windows. Sur la rangée des chiffres, tu vois donc `&`, `é`, `"`… parce que c'est ce qu'elle tape sans Maj. Pour de vrais chiffres, prends ceux du groupe « Pavé numérique ».

## 4. Appui court, appui long, double appui

Une touche **action** peut avoir jusqu'à trois actions, une par onglet :

| Onglet | Se déclenche quand… |
|---|---|
| **Appui court** | tu appuies normalement |
| **Appui long** | tu restes appuyé plus de 300 ms |
| **Double** | tu appuies deux fois en moins de 250 ms |

Si une touche a un appui long ou un double appui, son appui court attend un peu, le temps de savoir lequel des trois tu fais. Les délais se règlent dans *Réglages*.

Exemple : sur une même touche, appui court = scène « Jeu », double = scène « Pause », appui long = démarrer ou arrêter le stream.

## 5. Les types d'action

### Raccourci clavier
- **Pour** : Ctrl+Maj+M (couper le micro dans Discord), Alt+F4, Win+Maj+S (capture d'écran)…
- **Comment** : type « Raccourci clavier », clique dans le champ, puis appuie sur la combinaison sur ton clavier.
- Les lettres suivent ta disposition : Ctrl+A donne bien Ctrl+A en AZERTY.

### Texte
- **Pour** : ton adresse e-mail, une signature, une réponse toute faite.
- **Comment** : type « Texte », puis écris le texte. Il est tapé d'un coup, accents compris.

### Macro
- **Pour** : enchaîner plusieurs actions. Par exemple : changer de scène OBS, attendre 400 ms, couper le micro.
- **Comment** : type « Macro », puis ajoute des étapes **Faire** (une action) et **Attendre** (une pause en ms). Les boutons Monter et Descendre changent l'ordre.

### Lancer (une appli) / Ouvrir (un fichier ou une URL)
- **Pour** : ouvrir Discord, Spotify, OBS, un dossier, un site.
- **Comment** : type « Lancer », puis « Parcourir » et choisis le `.exe`. Les arguments se mettent un par ligne.
- Pour une URL (`https://…`) ou un fichier, coche **« URL ou fichier à ouvrir »** : il s'ouvre avec le programme par défaut.
- **Plus rapide** : glisse le `.exe` ou le fichier depuis l'Explorateur sur la touche (section 8).

### Commande
- **Pour** : un script PowerShell, un `.bat`, une commande système.
- **Comment** : type « Commande », puis écris la ligne, par exemple `powershell -File C:\scripts\backup.ps1`.

### Son (soundboard)
- **Pour** : jouer un son dans ton casque **et** dans ton micro (Discord, jeu), à la place de Soundpad.
- **Mise en place, une seule fois** :
  1. Installe **VB-Cable**. Si le moteur ne le trouve pas, son icône propose de l'installer.
  2. Dans Discord, le jeu ou OBS, choisis **« CABLE Output »** comme micro.
  3. Règle les sorties dans *Réglages → Audio*.
- **Comment** : type « Son », puis choisis le son, son volume, et ce qui se passe si tu rappuies pendant qu'il joue (« Relancer » ou « Couper »).
- **Plus rapide** : glisse un `.mp3`, `.wav`, `.ogg` ou `.flac` sur la touche.
- Dans la même famille : « Tout arrêter », « Volume sons » et « Couper sons ».

### OBS
- **Mise en place, une seule fois** :
  1. Dans OBS, va dans *Outils → Paramètres du serveur WebSocket*, coche « Activer le serveur WebSocket » (port 4455) et copie le mot de passe.
  2. Dans nullctl, va dans *Réglages → OBS*, colle le mot de passe et clique sur « Tester ».
- **Comment** : type « OBS », puis choisis le sous-type :

| Sous-type | Champ | Effet |
|---|---|---|
| **Scène** | nom exact de la scène, majuscules comprises | passe sur cette scène |
| **Source** | nom exact de la source audio | coupe ou rétablit cette source |
| **Stream** | Démarrer / Arrêter / Basculer | lance ou coupe le stream |
| **Enreg.** | Démarrer / Arrêter / Basculer | lance ou coupe l'enregistrement |
| **Replay** | — | sauvegarde le replay ; le « Tampon de relecture » doit être démarré dans OBS |

- Si OBS est fermé ou le nom faux, rien ne se passe et un avertissement est écrit dans les journaux (`%APPDATA%\nullctl\logs`). Le pad ne se fige pas.

### Volume et micro
- **Volume Windows** : monte ou baisse le volume général du pas choisi.
- **Couper Windows** : coupe ou rétablit le son.
- **Volume micro** et **Couper micro** : la même chose pour ton micro. Pratique sur une molette (section 6).

### MIDI (MAO : Ableton, FL Studio…)
- **Mise en place, une seule fois** :
  1. Installe [loopMIDI](https://www.tobias-erichsen.de/software/loopmidi.html), crée un port nommé **`NULL-PAD`** et laisse loopMIDI ouvert.
  2. Dans nullctl, va dans *Réglages → MIDI* et clique sur « Tester ».
  3. Dans Ableton, va dans *Paramètres → Link, Tempo & MIDI* et, sur l'entrée `NULL-PAD`, active **Remote** (pour mapper des boutons). Active aussi **Track** pour jouer un instrument.
- **Comment** : type « MIDI », puis choisis le sous-type :
  - **Note** : canal (1–16), note (0–127, 60 = do central), vélocité (1–127). C'est une frappe : la note n'est pas tenue tant que tu restes appuyé.
  - **CC** : canal, numéro de CC, valeur (0–127). Pour mapper un bouton d'Ableton : passe Ableton en mode MIDI Map (Ctrl+M), clique sur le bouton à piloter, puis appuie sur la touche du pad.

### Média
- **Lecture**, **Préc.**, **Suiv.** ou **Stop** pour Spotify, YouTube, etc.

### Bascule
- **Pour** : alterner deux actions, une à chaque appui. Par exemple : 1er appui = scène « Jeu », 2e appui = scène « Caméra ».
- **Comment** : type « Bascule », puis choisis l'action **Allumé** et l'action **Éteint**.

## 6. Les molettes

Chaque molette a trois entrées : **Sens horaire**, **Sens anti-horaire** et **Clic**. Chaque cran compte comme un appui. Clique sur la molette dans le pad 3D pour les configurer, comme une touche.

| Exemple | Sens horaire | Sens anti-horaire | Clic |
|---|---|---|---|
| Volume (natif, par défaut à gauche) | Volume + | Volume − | Muet |
| Micro (par défaut à droite) | Volume micro +2 | Volume micro −2 | Couper micro |
| Ableton | CC 20, valeur 1 | CC 20, valeur 127 | Note ou CC au choix |

Pour Ableton : mappe ce CC en mode **« Relative (2's Comp.) »**. Ici, 1 et 127 sont les *valeurs* envoyées, pas le numéro de CC.

## 7. Les profils

- Un **profil** est un jeu complet de touches. **M1, M2 et M3** passent aux profils 1, 2 et 3. Tu peux créer jusqu'à **8 profils** avec « Nouveau profil ». Les profils 4 à 8 n'ont pas de touche M : on y va avec une touche native réglée sur « Aller au profil ».
- Le **profil 1 (DEF)** est le pavé numérique natif : il marche même sans nullctl.
- Le **tag** (3 lettres ou chiffres, par exemple `OBS`, `MAO`) s'affiche sur l'écran du pad et se change dans le profil.
- La **description** (facultative, 140 caractères) s'affiche sur l'accueil, dans la fenêtre `profil_<tag>.exe`, sous le nom du profil. Tu la remplis dans « Nouveau profil » ou dans « Configurer », à côté du nom et du tag.
- **Supprimer le profil…** retire le profil du pad et de la config.

Idée de départ :

| Profil | Tag | Usage |
|---|---|---|
| M1 | `DEF` | pavé numérique |
| M2 | `OBS` | scènes, micro, stream, sons |
| M3 | `MAO` | notes et CC pour Ableton |

## 8. Glisser-déposer

En mode Configurer, glisse un fichier depuis l'Explorateur sur une touche ou une molette :

| Fichier | Devient |
|---|---|
| `.mp3`, `.wav`, `.ogg`, `.flac` | Son |
| `.exe`, `.bat`, `.cmd` | Lancer ce programme |
| `.ps1` | Lancer le script avec PowerShell |
| autre fichier, raccourci `.lnk` compris | Ouvrir |

Seul le premier fichier déposé est utilisé.

## 9. Recettes

**Streamer (profil OBS)**
- Rangée du haut : 4 scènes OBS (Jeu, Caméra, Pause, Fin).
- `+` : Bascule, avec Stream « Démarrer » / Stream « Arrêter ».
- `Entrée` : appui court = Replay ; appui long = Enreg. « Basculer ».
- Pavé 1–9 : 9 sons de la soundboard.
- Molette droite : volume micro, et clic pour couper le micro.

**MAO (profil MAO)**
- Les 16 touches du pavé : notes 36 à 51 (comme un pad de batterie : 36 = grosse caisse dans le Drum Rack).
- Les deux molettes : CC 20 et CC 21 en relatif, mappées sur un filtre et une réverb.
- `Entrée` : CC 64 valeur 127, mappé sur Play.

**Bureau**
- Lancer : navigateur, Discord, Spotify, Explorateur.
- Raccourcis : Win+Maj+S, Ctrl+Maj+Échap, Win+D.
- Texte : adresse e-mail, signature.

## 10. Problèmes fréquents

| Problème | Solution |
|---|---|
| La touche tape `q` au lieu de `a` | La touche a été choisie avec une préversion, avant la correction AZERTY : choisis-la de nouveau dans la liste. |
| « Pad verrouillé » | Appuie sur M1 + M3 ensemble. Le pad se reverrouille quand tu le débranches ou après 10 min sans activité. |
| « Pad non branché », « Lecture seule en Bluetooth » ou touches grisées | Branche en **USB**. L'écriture ne passe pas par le Bluetooth. Ferme ZMK Studio. |
| « Pad modifié ailleurs » | Clique sur **Recharger**. Si ZMK Studio a laissé des changements, clique sur « Abandonner les changements du pad ». |
| Une action ne fait rien | Le moteur tourne-t-il (icône dans la zone de notification) ? Regarde `%APPDATA%\nullctl\logs`. |
| OBS ne répond pas | Serveur WebSocket activé ? Bon mot de passe dans *Réglages → OBS* ? Nom de scène exact ? |
| MIDI : rien dans Ableton | loopMIDI ouvert avec le port `NULL-PAD` ? Remote ou Track activé dans Ableton ? |
| Le son ne passe pas dans Discord | Micro de Discord = « CABLE Output ». |
