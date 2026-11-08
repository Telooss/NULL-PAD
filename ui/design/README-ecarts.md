# Ecarts avec l'artefact nullctl

Les fichiers de ce dossier sont la copie telle quelle de l'artefact. Les fichiers servis par l'appli, dans `ui/web/`, en different ainsi :

- `web/css/bundle.css` : l'`@import` Google Fonts de la premiere ligne est remplace par des `@font-face` locaux (`web/fonts/`, woff2 latin). Le reste est identique a `design/components/bundle.css`.
- `web/js/bundle.js` : copie identique de `design/components/bundle.js` (il doit etre sous `web/`).
- `web/css/tokens.css` : variables `:root` et classes de texte (`.ui-small`, ...) ecrites depuis `tokens.json`. Les previews de l'artefact les recevaient de la plateforme.
- `web/css/app.css` : `.nc-app` prend `100vw x 100vh` au lieu de 1440x900 fixes, plus un survol des boutons de fenetre.
- `web/assets/nullpad.json` : copie de `design/assets/Modele/nullpad.json` (charge a l'execution, Task 7).
- `web/vendor/` : three.js 0.160.0 et lucide 0.344.0 locaux, a la place des CDN des previews.
