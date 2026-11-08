# Interface nullctl — points de suivi

Relevés par les relectures (2026-11-08), non bloquants.

## À vérifier avec le vrai pad
- Appuis en direct, USB et BLE (heuristique `BTHLE` / `{00001812-` du chemin de périphérique).
- Glisser-déposer réel dans la fenêtre Tauri (seulement testé avec le dépôt simulé).
- GPU au repos dans le Gestionnaire des tâches (aucune boucle de rendu, mais pas mesuré).

## Mineurs
- La liste des scènes et sources OBS n'est pas lue dans OBS : saisie libre.
- L'appli vérifie seulement le nom de l'exécutable qui tient le pipe (pas son chemin complet).
- `configWaiter` prend le premier événement `config`, quelle que soit sa source.
- L'appui d'un autre profil allume la touche sur le rendu du profil affiché.
- Un Raccourci vide est accepté par le moteur (échoue à l'appui).
- Un son déposé reste dans la bibliothèque après le retrait de son action.
- Polices en sous-ensemble latin ; icône d'appli provisoire.
- `dispose` du pad ne libère pas textures et géométries (pad créé une seule fois).
- Les fils Macro / Lancer n'ont pas de `catch_unwind` : pas d'événement `action` si panique.
