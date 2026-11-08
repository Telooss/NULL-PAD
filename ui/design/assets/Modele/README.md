La géométrie du vrai NULL/PAD (`case/model`, build123d → STL → JSON), chargée en direct par three.js dans les maquettes.

- `nullpad.json` : 71 pièces en place (coque givrée, keycaps, molettes, cadre d'écran, switches, encodeurs, module, écran, et le vrai PCB tiré du STEP KiCad avec ses perçages), 26 formes uniques. Chaque forme est quantifiée en uint16 (`ext` = taille en mm) et chaque pièce est une forme + un décalage `at`. 1,6 Mo. Les composants du PCB sont tous au dos et invisibles depuis le dessus : ils ne sont pas inclus.
- Les matières sont choisies par nom de pièce dans `NullCtl.pad3d` : violet translucide pour les molettes, le cadre `C`, `F20` et `F21`–`F23` ; **PCB en vernis violet**, visible à travers la coque givrée (`A1`, `A2`, `B`, opacité 0,7).
- Si le modèle change, il faut refaire ce fichier (export des pièces en place, puis empaquetage). Les zones cliquables, les légendes et l'écran suivent tout seuls.
