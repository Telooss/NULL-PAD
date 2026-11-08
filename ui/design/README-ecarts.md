# Ecarts avec l'artefact nullctl

Les fichiers de ce dossier sont la copie telle quelle de l'artefact. Les fichiers servis par l'appli, dans `ui/web/`, en different ainsi :

- `web/css/bundle.css` : l'`@import` Google Fonts de la premiere ligne est remplace par des `@font-face` locaux (`web/fonts/`, woff2 latin). Le reste est identique a `design/components/bundle.css`.
- `web/js/bundle.js` : copie de `design/components/bundle.js` (il doit etre sous `web/`), avec ces modifications de `pad3d`, marquees `nullctl` dans le code (Task 7, regles de performance du spec §6) :
  - densite de pixels plafonnee a 1,5 au lieu de 2 ;
  - `render()` ne dessine plus directement : il demande une image via un seul `requestAnimationFrame` en attente (les appels se regroupent) ; rien n'est dessine si `document.hidden`, et un `visibilitychange` redemande une image au retour ;
  - textures de legende et du nice!view redessinees seulement si leur contenu change ; une legende peut etre `[l1, l2]` (deux lignes, 50 px) ;
  - placement de la camera dans `aim()`, refait par `resize(w, h)` ;
  - survol factorise dans `hoverOn()`, plus un `pointerleave` qui l'efface ;
  - nouvelles fonctions renvoyees : `hover(piece)`, `press(piece, down, dz=1.5)` (enfoncement le long de z avec la legende et le contour, lueur violette), `pick(x, y)`, `resize`, `dispose`, `invalidate`, `frames()`.
- `web/css/tokens.css` : variables `:root` et classes de texte (`.ui-small`, ...) ecrites depuis `tokens.json`. Les previews de l'artefact les recevaient de la plateforme.
- `web/css/app.css` : `.nc-app` prend `100vw x 100vh` au lieu de 1440x900 fixes, plus un survol des boutons de fenetre.
- `web/assets/nullpad.json` : copie de `design/assets/Modele/nullpad.json` (charge a l'execution, Task 7).
- `web/vendor/` : three.js 0.160.0 et lucide 0.344.0 locaux, a la place des CDN des previews.
