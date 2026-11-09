# Écriture dans le pad depuis nullctl (ZMK Studio RPC) — conception

Date : 2026-11-08. Statut : **à valider par Alan**. Elle complète `2026-11-07-interface-design.md` (§8 mode configuration, §12) et `2026-11-05-logiciel-macropad-design.md` (§3.1, choix « notre logiciel parle le protocole RPC de ZMK Studio »).

## 1. Intention

**Tout le NULL/PAD se règle depuis nullctl** : toutes les touches, M1–M3, les deux molettes, et le tag de 3 lettres de chaque profil. Le pavé numérique n'est que le réglage de départ du firmware. Ce que vit dans le pad (touches natives) et ce que vit dans le logiciel (actions du moteur) se règlent au même endroit, dans le même mode configuration, avec un seul « Enregistrer » (décision d'Alan, 2026-11-08 : « on va pas faire compliqué »).

## 2. Objectifs, non-objectifs, anti-objectifs

**Objectifs**
- Avec le pad branché en USB, chaque entrée (`K00`–`K19`, `M1`–`M3`, molettes : sens horaire, anti-horaire, clic) de chaque profil peut être :
  - une **touche native** : touche clavier, pavé, média, combinaison, « Aller au profil N », ou une macro de profil du firmware (`np_p1`–`np_p8`) ;
  - une **action nullctl** (son, OBS, macro, commande…) : le pad envoie alors le code action `ACT(profil, entrée)` et le moteur fait le reste.
- Renommer un profil met aussi à jour le **nom de la couche** dans le pad, donc le tag affiché sur l'écran.
- Les légendes du mode vue montrent la **vraie** fonction des touches, lue dans le pad quand il est branché.

**Non-objectifs (cette version)**
- Écrire dans le pad en Bluetooth : ZMK Studio n'y est pas utilisable depuis une appli Windows comme la nôtre.
- Changer le nombre de couches, les déplacer, ou modifier la combo de déverrouillage M1 + M3.
- Comportements avancés de ZMK (hold-tap, tap-dance…) en dehors de ceux que le firmware déclare déjà.

**Anti-objectifs (échec même si ça marche)**
- Un pad qu'on ne peut plus déverrouiller ou qui n'a plus de touche de profil, à cause d'une modification faite dans nullctl.
- Une modification du pad faite ailleurs (ZMK Studio) écrasée sans prévenir.
- Une config et un pad en désaccord sans que l'utilisateur le sache.

## 3. Protocole (vérifié dans `zmk-studio-messages` @ `6cb4c283`, épinglé par ZMK v0.3.0, et `app/src/studio/` de ZMK v0.3.0)

- **Transport** : le port série USB CDC-ACM du pad (snippet `studio-rpc-usb-uart`, déjà dans notre build). On le trouve par son VID/PID `0x1D50` / `0x615E`.
- **Trames** : début `0xAB`, échappement `0xAC` (l'octet suivant est pris tel quel), fin `0xAD`. Chaque trame contient un message protobuf.
- **Messages** (`studio.proto`) :
  - `Request { request_id, oneof core | behaviors | keymap }` ;
  - `Response { oneof request_response | notification }`.
- **Requêtes utilisées** :
  - `core` : `get_device_info`, `get_lock_state` (+ notification `lock_state_changed`) ;
  - `behaviors` : `list_all_behaviors`, puis `get_behavior_details(id)` → `display_name` et métadonnées des paramètres. Les ids de comportement sont attribués à l'exécution : on les associe par nom (« Key Press », les macros `np_*`…) ;
  - `keymap` : `get_keymap` (couches `{id, name, bindings[]}`, `available_layers`, `max_layer_name_length`), `set_layer_binding {layer_id, key_position, binding{behavior_id, param1, param2}}`, `set_layer_props {layer_id, name}`, `check_unsaved_changes`, `save_changes`, `discard_changes`.
- **Correspondances** :
  - la position de touche du firmware est **l'`InputId` du moteur** (0–28, `firmware/null_pad_codes.h`) ;
  - le profil `p` est la couche d'**indice** `p-1` de `get_keymap` (l'`id` de couche se lit dans la réponse) ;
  - un `&kp` prend en `param1` l'usage HID complet, `(page << 16) | id` ;
  - une action nullctl est `&kp ACT(p, entrée)`, avec la page consommateur `0x0C`.

## 4. Comportement dans l'interface

- **État du pad, toujours visible** dans la barre des profils, à côté de la pastille USB/BLE :
  - « Pad modifiable » quand il est branché en USB et que la keymap est lue ;
  - « Pad en lecture seule (Bluetooth) » ;
  - « Pad non branché ».
- **Lecture** : à la connexion USB, et à chaque entrée en mode configuration, l'appli lit la keymap du pad et la garde en mémoire. Les légendes de chaque profil viennent de cette lecture, combinée avec les actions du moteur :
  - si la touche du pad est `&kp ACT(p, entrée)`, la légende est celle de l'action nullctl ;
  - sinon, c'est la touche native (« 7 », « VOL+ », « P2 »…).
  
  Sans pad branché, on garde la table du firmware par défaut (comportement actuel).
- **Mode configuration**, pad branché en USB :
  - **Plus aucune touche n'est en lecture seule**, sauf la combo de déverrouillage : M1 et M3 restent modifiables un par un, mais au moins une touche « Aller au profil 1 » doit rester quelque part (garde-fou, voir plus bas).
  - « Changer d'action » propose une nouvelle famille, **Touche native**, avec un sélecteur :
    - clavier ;
    - pavé numérique ;
    - média et volume ;
    - combinaison (Ctrl, Maj, Alt, Win + touche) ;
    - « Aller au profil N » (`&to`) ;
    - les macros de profil `np_p1`–`np_p8`.
  - **Passer une touche en native** retire les actions nullctl de cette entrée pour ce profil, quel que soit le déclencheur : elles ne pourraient plus jamais se déclencher.
  - **Passer une touche native en action nullctl** écrit `&kp ACT(p, entrée)` dans le pad, puis l'action dans la config.
  - **Renommer le profil** change aussi le nom de la couche du pad. Le nom de couche reçoit le **tag** (3 caractères) et pas le nom long : c'est ce que l'écran affiche.
- **Mode configuration, pad non branché en USB** : les choix « Touche native » sont grisés, et les touches natives restent en lecture seule, avec « Branchez le pad en USB pour modifier ». Les actions nullctl sur les touches qui envoient déjà un code action restent modifiables, comme aujourd'hui.
- **Glisser-déposer sur une touche native**, pad branché : la touche devient une action nullctl (même règle que ci-dessus), plutôt que le refus actuel.

## 5. Enregistrer

L'ordre est fixe, et chaque étape s'arrête proprement si elle échoue.

1. **Validation** de la config entière (existant).
2. **Pad modifié dans le brouillon ?**
   - Si oui : relecture du pad et comparaison avec la lecture du début de la session. Si le pad a changé entre-temps, par exemple dans ZMK Studio, on s'arrête : « Le pad a été modifié ailleurs. Recharger ? ». Rien n'est écrit.
   - Ensuite, verrou : si `get_lock_state` répond verrouillé, on affiche « Appuie sur M1 + M3 pour autoriser l'écriture » et on attend la notification `lock_state_changed`, une minute au plus. Si c'est annulé ou trop long, rien n'est écrit.
3. **Écriture de la config** (existant : atomique, puis attente de l'événement `config` du moteur).
4. **Écriture du pad** : les `set_layer_binding` et `set_layer_props` du brouillon, puis `save_changes`.
   - Si une étape échoue, on appelle `discard_changes` et on affiche « Config enregistrée, mais le pad n'a pas été modifié : <raison>. Réessayer l'écriture dans le pad ? ». Le brouillon du pad est gardé pour réessayer.
   - On n'annule pas la config : si une action nullctl a été ajoutée sans son code dans le pad, elle ne se déclenche pas, mais elle n'abîme rien. La pastille passe en « Pad pas à jour ».
5. **Succès** : les légendes sont relues depuis le pad.

**Garde-fous vérifiés avant l'étape 3** (refus avec message, rien n'est écrit) :
- il reste au moins une touche « Aller au profil 1 » (`&to 0` ou `np_p1`) accessible depuis chaque profil, ou M1 n'est pas modifiée ;
- la combo M1 + M3 → `&studio_unlock` n'est jamais modifiée (de toute façon, Studio ne l'expose pas) ;
- un nom de couche fait au plus `max_layer_name_length` caractères.

## 6. Architecture

- **Rust, côté interface** (`ui/src/studio/`) :
  - le client RPC, avec le port série (crate `serialport`), les trames et les messages protobuf générés à la compilation depuis les `.proto` de `zmk-studio-messages` (`prost` + `prost-build` ; le `protoc` vient de `protoc-bin-vendored`, pour ne rien installer) ;
  - les `.proto` sont copiés dans le dépôt (`ui/proto/`), au commit épinglé, avec leur licence ;
  - un seul client à la fois, ouvert à la demande et fermé quand l'appli n'en a plus besoin, pour ne pas bloquer ZMK Studio.
- **Commandes Tauri** : `pad_status`, `pad_read` (keymap traduite en entrées et profils), `pad_unlock_wait`, `pad_write(brouillon_pad)`, `pad_discard`. La logique de traduction (keymap ↔ entrées, usages HID ↔ noms de touches) est **pure et testée**.
- **Côté web** : le brouillon du mode configuration gagne une partie « pad » (les changements de liaisons et de noms de couche). `saveFlow` gagne l'étape pad, testée comme les autres issues.
- **Le moteur ne change pas** : il continue de recevoir les codes action. La détection USB/BLE du moteur (Task 2 du plan interface) alimente la pastille.

## 7. Tests

- **Purs (sans pad)** :
  - trames : encodage, échappement de `0xAB`/`0xAC`/`0xAD`, trame coupée en morceaux ;
  - correspondance keymap ↔ entrées ;
  - usages HID ↔ noms ;
  - garde-fous du §5 ;
  - toutes les issues de `saveFlow` avec l'étape pad.
- **Faux pad** : un faux serveur RPC en mémoire (un « port » simulé), qui répond comme le firmware à `get_keymap`, `set_layer_binding`, `save_changes`, au verrou et aux notifications. Il couvre : lecture, écriture, verrou, erreur d'écriture → `discard_changes`, pad modifié ailleurs.
- **Vrai pad** : `#[ignore]`, à lancer à la main. Ils sont listés dans `ui/README.md`.

## 8. Laissé au builder

- La présentation du sélecteur de touche native (liste, recherche, capture au clavier).
- Le découpage du code Rust.
- La durée exacte des délais (lecture, verrou), dans la limite d'une minute pour le verrou.
- La liste précise des touches proposées, à prendre dans `keys.h` de ZMK v0.3.0.

## 9. Points tranchés après la relecture du builder (choix de Claude)

1. **Le verrou bloque aussi la lecture.** Dans ZMK v0.3.0, `keymap` et `behaviors` sont protégés : seuls `get_device_info` et `get_lock_state` répondent sur un pad verrouillé. Le pad démarre verrouillé et se reverrouille à la déconnexion ou après 600 s d'inactivité.
   - **Mode vue** : on affiche la **dernière keymap lue**, mise en cache dans `%APPDATA%\nullctl\pad-cache.json` avec le numéro de série du pad. Sans cache, c'est la table du firmware par défaut. Pastilles : « Pad verrouillé » ou « Pad modifiable ».
   - **Entrée en mode configuration avec le pad en USB** : on demande « Appuie sur M1 + M3 » une fois, puis on lit et on met à jour le cache. Si l'utilisateur refuse, le mode configuration fonctionne comme sans pad : les touches natives sont en lecture seule.
   - **Reverrouillage pendant un brouillon** (600 s) : au moment d'enregistrer, on redemande M1 + M3 si nécessaire. La comparaison « pad modifié ailleurs » se fait avec la lecture faite à l'entrée.
2. **Profils et couches du pad.** Les opérations sur les profils passent par la même étape pad (verrou, puis écriture, puis `save_changes`), quand le pad est branché en USB et déverrouillé.
   - **Créer** : la couche reçoit les liaisons par défaut du firmware (grille en `ACT(p,·)`, `np_p1..3`, molette gauche en volume, molette droite en `ACT`) et le tag comme nom.
   - **Copier depuis un profil** : on copie aussi ses liaisons natives.
   - **Supprimer** : la couche revient aux liaisons par défaut, et son nom devient « P0N ».
   - **Sans pad** : l'opération se fait dans la config seulement, et la pastille passe en « Pad pas à jour ». Il n'y a pas de file d'attente.
3. **« Aller au profil N »** ne propose que les macros `np_p1..np_p8`, qui préviennent le moteur. Le `&to` seul n'est pas proposé. Garde-fou : chaque couche doit garder au moins une touche `np_p1`.
4. **Molette gauche** : le module `np_enc` du firmware transforme chaque cran en appui sur les positions cachées 23/24 (26/27 à droite). Changer la liaison de ces positions change donc bien l'effet de la rotation. Le clic, aux positions 25 et 28, est une touche normale.
5. **Changements non enregistrés dans le pad à la connexion** (`check_unsaved_changes` vrai, par exemple laissés par ZMK Studio) : nullctl refuse d'écrire et propose « Le pad a des changements non enregistrés faits ailleurs : les abandonner ? » (`discard_changes`, après confirmation). Il ne fait jamais de `save_changes` qui enregistrerait des changements étrangers.
6. **Ordre de l'enregistrement quand le brouillon touche le pad** : **le pad d'abord, la config ensuite**, après une validation complète des deux.
   - Si le pad échoue : `discard_changes`, rien n'est écrit, le brouillon est gardé.
   - Si la config échoue après la réussite du pad (rare, puisqu'elle est validée avant) : « Pad mis à jour, config non enregistrée », et l'enregistrement de la config se réessaie avec le même brouillon.
   - Ce choix remplace l'ordre du §5 : on ne perd jamais d'actions nullctl à cause d'un pad qui refuse.
7. **« Pad modifiable » ne dépend que du port série** trouvé par VID/PID et qui répond à `get_device_info`. La pastille USB/BLE du moteur reste à part.
