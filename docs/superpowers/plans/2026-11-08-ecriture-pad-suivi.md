# Écriture dans le pad : points de suivi

Plan : [2026-11-08-ecriture-pad.md](2026-11-08-ecriture-pad.md). Les 6 tâches ont été faites et relues une à une, puis toute la branche a été relue. Tests : Rust et clippy sans avertissement, web 81/81. Rien n'a encore été essayé sur le vrai pad.

## À vérifier sur le vrai pad (liste complète dans `ui/README.md`)

- **Le pad répond au port série.** `open_pad` lève le signal DTR à l'ouverture. Si chaque appel expire, c'est le premier endroit à regarder.
- **Le pad se reverrouille-t-il quand nullctl ferme le port ?** Si oui, chaque lecture puis chaque écriture redemande M1 + M3. Il faudra alors garder le port ouvert pendant tout le mode configuration.
- **Reverrouillage après 10 min, nullctl ouvert.** La pastille ne fait plus que lister les ports, sans en ouvrir aucun. Le minuteur du pad ne devrait donc plus être relancé.
- **Noms des comportements.** nullctl suppose que les macros du firmware s'appellent « np_p1 » … « np_p8 » et « np_00 », et la frappe de touche « Key Press ». Ces noms ont été déduits du keymap.
- **`set_layer_props` et `max_layer_name_length`.** Ils ont seulement été testés contre le faux pad.

## Améliorations possibles (non bloquantes)

| Point | Où |
|---|---|
| Après un `save()` qui expire alors que le pad a bien enregistré, la tentative suivante dit « modifié ailleurs ». On pourrait vérifier avec `check_unsaved` et une relecture. | `ui/src/studio/service.rs` |
| `guard` vérifie toutes les couches. Une couche cassée dans Studio, sur un profil inexistant, bloque donc l'écriture. Il faudrait ne vérifier que les couches modifiées. | `ui/src/studio/model.rs` |
| Après « Recharger », réappliquer les touches passées en action au lieu de les retirer. | `ui/web/js/app.js` |
| Ajouter « Enregistrer sans le pad » quand le pad est débranché en cours de brouillon (config seule, pastille « Pad pas à jour »). | `ui/web/js/pad-flow.js` |
| Carte « Touche native » grisée au lieu d'être masquée quand le pad est en lecture seule. | `ui/web/js/editpanel.js` |
| `pad_read` et `pad_write` bloquent un fil tokio jusqu'à 60 s. Ça passe, avec un seul client à la fois. Utiliser `spawn_blocking` serait plus propre. | `ui/src/studio/commands.rs` |
| `pad-flow.js` n'a pas de tests directs (la partie pure est dans `pad-model.js`). | `ui/web/test/` |
| « Pad pas à jour » peut rester affiché si un profil supprimé a déjà une couche identique à la couche par défaut. | `ui/web/js/pad-flow.js` |
| Studio en Bluetooth pendant une écriture USB : des changements pourraient se glisser entre la vérification et l'enregistrement. La fenêtre est minuscule. | — |
