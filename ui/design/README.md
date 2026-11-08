nullctl est l'appli de réglage du NULL/PAD. Son identité : néo-punk hacktiviste × années 2000, **sans kitsch**. L'outil reste net, et l'identité vit dans le décor, dans la typo des noms et dans de rares événements. Ce sont des maquettes de direction artistique, pas des écrans figés : commente-les directement.

## Le test

Coupe tous les effets (grain, trame, fenêtres du décor, glitch) : l'appli doit rester parfaitement claire et utilisable. Sinon, l'effet est mal placé.

## Ce qu'on ne fait jamais

- Néon-graffiti, chrome clair, riso ou autocollants en plein écran, interface au trait façon croquis.
- Hello Kitty, cœurs, rose, étoiles partout (une étoile à 4 branches visible au maximum).
- Crâne ou tout symbole qui imite DedSec ou Watch Dogs. L'identité passe par le nom et sa typo.

## Noms

- **NULL/PAD** : le macropad. En capitales, avec le slash. Il est imprimé sur le rendu du pad et apparaît dans l'en-tête.
- **nullctl** : le logiciel. En minuscules, se dit « null control ». Il apparaît dans la barre de titre, sur l'icône et à l'accueil. Le mot-symbole s'écrit `wordmark` : « null » en `ink`, « ctl » en `violet-400`, suivi d'un curseur plein `violet-500`.
- Les deux noms sont provisoires.

## Structure, toujours la même

1. **Barre de titre** (36 px) : nullctl, puis « — NULL/PAD ».
2. **Barre des profils** (76 px) : les emplacements M1–M3 en haut à gauche. À droite, en permanence : la connexion (USB / BLE), l'état d'enregistrement et le bouton Réglages (les réglages globaux sont sur un écran à part).
3. **Scène** : le NULL/PAD au centre, grand, rendu depuis le vrai modèle 3D. On clique une touche, une molette ou sa légende.
4. **Panneau de réglage** : 400 px, toujours à droite, un seul ouvert à la fois. Un fil coudé `violet-400` relie la touche sélectionnée au titre du panneau. La mini-grille du titre montre la même touche.

## Couleurs

- Un seul accent : le violet du pad. `violet-500` pour les pleins (bouton principal, autocollant de profil), `violet-400` pour le texte, les icônes, le fil et le contour de sélection sur fond sombre. Pas de vert ni de deuxième accent.
- Fonds : `bg-000` pour la fenêtre, `bg-100` pour la scène, `surface-100` pour le panneau et les barres, `surface-200` au survol.
- Texte : `ink`, et `ink-muted` pour le secondaire. `ink-faint` sert uniquement au décor (registre, code-barres) et ne porte jamais d'information.
- Une sélection, c'est `violet-900` en fond plus un bord `violet-500`.
- `win-face`, `win-ink` et `tape` servent exclusivement aux fenêtres du décor.

## Typo

- Interface en `sans` (IBM Plex Sans) : `ui-title`, `ui-body`, `ui-strong`, `ui-small`.
- Machine en `mono` (IBM Plex Mono) : `mono-label` pour les petits libellés en capitales, `mono-key` pour le nom de la touche.
- `sticker` (Bagel Fat One), en capitales, avec un contour `ink` et une ombre dure `violet-700`, est réservé aux **noms de profil** : `profile-sticker` dans la barre, `profile-hero` à la création. Jamais sur un bouton ni un libellé.
- `lcd` (Silkscreen), uniquement pour ce qu'affiche l'écran nice!view du rendu.

## Le décor « terminal »

Tout ce qui suit reste dans la scène, jamais dans le panneau ni dans la barre des profils :

- **Cadre noir mat** : coins en équerre `ink-muted`, trame de points `line-200` qui n'apparaît qu'en fondu vers les bords.
- **Petits textes techniques** : registre vertical en mono capitales (« 0x4E50 // NODE 01 // … ») et code-barres avec sa référence (« NP-01-DFLT-0001 »).
- **Étoile** : une étoile à 4 branches au plus.
- **Fenêtres Win98 scotchées** : barre de titre `violet-700` → `violet-500`, texte qui raconte ce que fait la machine (« Chargement du profil DEFAULT… »). Ne mettre aucune vraie information dedans.
- **Grain de fond** : sur toute la fenêtre, très léger.

## Glitch

Il ne se déclenche que sur un événement : changement de profil, sauvegarde, création de profil. Il dure 0,36 s (décalage RGB, saccade, lignes de balayage) puis disparaît. Il ne masque jamais une information, et il est coupé si Windows demande moins d'animations.

## Actions

Chaque action a une icône (Lucide, trait 1,75) **et** un libellé, plus une ligne de description en `ui-small`. L'action en cours est affichée en tête du panneau, avant la liste « Changer d'action ». Les déclencheurs (appui court, appui long, double, bascule) sont des onglets segmentés sous le titre.

## Le pad en 3D

Le NULL/PAD est une scène three.js en direct, construite à partir de `assets/Modele/nullpad.json` (le vrai modèle `case/model`, switches et électronique compris, pour que la coque givrée ne laisse pas voir de trous). La caméra a une focale longue (16°) et une inclinaison de 24° vers l'utilisateur.

- Les légendes des touches, le tag de l'écran nice!view et « NULL/PAD » sont des **textures posées sur les surfaces**. Ils suivent la perspective et la rotation, et ne sont jamais du texte superposé.
- La sélection se voit par les arêtes de la keycap en `violet-400` plus une légère lueur violette. Le survol donne une lueur plus faible. Le clic passe par un lancer de rayon sur les keycaps et les molettes.
- Les polices doivent être chargées avant de dessiner une texture (`NullCtl.fontsReady()`).
