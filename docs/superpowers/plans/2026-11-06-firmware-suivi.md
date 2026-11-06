# Firmware — points de suivi

Relevés par les relectures (2026-11-06), non bloquants.

## À vérifier sur le vrai pad
- Chaque cran de molette est un appui de 0 ms : vérifier que le volume bouge à chaque cran en USB et en Bluetooth. Sinon, relâcher après un court délai (`k_work_delayable`) dans `behavior_np_sensor_keys.c`.
- Sens de rotation (sinon inverser A/B dans l'overlay), `steps` / `triggers-per-rotation`, anti-rebond 1 ms.
- La combo M1 + M3 (déverrouillage Studio) peut retarder M1 et M3 jusqu'à 50 ms.

## Mineurs
- `np_enc` apparaît dans la liste des comportements de Studio ; l'affecter à une touche ne fait rien.
- Test des codes : une valeur de `#define` non lisible donne un message « manquant » trompeur ; `N_INPUTS = 29` est écrit en dur.
- `build-local.sh` réinstalle les dépendances Python et recompile tout à chaque lancement (`-p auto` irait plus vite).
- GitHub Actions bloqué par la facturation du compte : le workflow `firmware.yml` reste prêt mais n'a jamais tourné.
