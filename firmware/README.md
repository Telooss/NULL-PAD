# Firmware du NULL/PAD

Firmware [ZMK](https://zmk.dev) **v0.3.0** pour le NULL/PAD (nice!nano v2, 4×5 touches, M1–M3, deux molettes, écran nice!view). Il garde un pavé numérique natif qui marche sans logiciel, et envoie au moteur les codes « action » et « annonce de profil » qu'il attend (voir `engine/README.md`).

## Récupérer le .uf2

**Compilation locale (WSL Ubuntu)**, depuis la racine du dépôt, dans PowerShell :

```powershell
wsl -e bash firmware/build-local.sh
```

Le résultat est `firmware\out\null_pad.uf2`. Le script copie le dépôt dans `~/np` (`src` et `ws`) (côté WSL), récupère ZMK et Zephyr avec west (premier passage long, ensuite rapide), compile et recopie le `.uf2`. Il peut être relancé autant de fois que nécessaire.

**Installation unique de la toolchain** (dans WSL Ubuntu) :

```bash
sudo apt update && sudo apt install -y git cmake ninja-build gperf ccache dfu-util \
  device-tree-compiler python3-dev python3-venv xz-utils file make gcc g++ protobuf-compiler rsync wget
python3 -m venv ~/zmk-venv && source ~/zmk-venv/bin/activate
pip install west "setuptools<81" protobuf grpcio-tools
cd ~ && wget https://github.com/zephyrproject-rtos/sdk-ng/releases/download/v0.16.8/zephyr-sdk-0.16.8_linux-x86_64_minimal.tar.xz
tar xf zephyr-sdk-0.16.8_linux-x86_64_minimal.tar.xz
cd zephyr-sdk-0.16.8 && ./setup.sh -t arm-zephyr-eabi -h -c
```

**Alternative, si les Actions sont disponibles** : le workflow `.github/workflows/firmware.yml` compile à chaque push sur `firmware/**`. Le `.uf2` est l'artefact `null_pad` de l'onglet Actions du dépôt.

## Flasher

1. Brancher le pad en USB, appuyer deux fois rapidement sur reset : le lecteur `NICENANO` apparaît.
2. Y copier `null_pad.uf2` ; le pad redémarre tout seul.
3. Si un ancien appairage Bluetooth gêne, flasher d'abord un firmware `settings_reset`, puis `null_pad.uf2`. Pour le compiler (dans l'espace WSL `~/np/ws`) : `west build -p -s zmk/app -d build-reset -b nice_nano_v2 -- -DSHIELD=settings_reset`, le `.uf2` est `~/np/ws/build-reset/zephyr/zmk.uf2`.
4. Bluetooth : le pad n'a pas de touche `&bt` ; assigner `&bt BT_SEL n` / `&bt BT_CLR` à une touche dans Studio si besoin.

## Profils

- **M1–M3** = profils 1–3. Les profils 4 à 8 existent aussi : les comportements `np_p4` à `np_p8` sont dans la liste de ZMK Studio, à assigner à la touche voulue.
- Le profil 1, « DEF », est un **pavé numérique natif** : il marche sans logiciel.
- Le tag affiché à l'écran est le **nom de la couche**, renommable dans Studio : 3 caractères A–Z ou 0–9, pour rester d'accord avec le moteur.

## Molettes

ZMK Studio ne sait pas éditer le comportement d'une molette. Le firmware ajoute donc, pour chaque molette, deux positions de touche qui n'existent pas physiquement (sens horaire et antihoraire) : chaque cran « appuie » sur la position du sens tourné, et couches, combos et Studio s'appliquent comme pour une vraie touche. Les clics de molette sont de vraies touches.

Par défaut, la molette gauche règle le **volume** en natif (sans logiciel) et la droite pilote le **micro** par le moteur. Tout est modifiable profil par profil.

## ZMK Studio

Pour les réglages courants (touches, molettes, profils, tag), **nullctl remplace ZMK Studio** : voir [../ui/README.md](../ui/README.md). Studio reste possible, mais pas en même temps que nullctl (un seul programme parle au pad).

- Déverrouiller avec **M1 + M3** appuyés ensemble.
- Connexion **par USB ou Bluetooth** (`ZMK_STUDIO_TRANSPORT_BLE=y`) ; USB recommandé.
- Les molettes apparaissent en haut de la disposition : 4 touches cachées et 2 clics, avec un espace entre molette gauche et droite.

## Codes

Les codes envoyés au moteur sont décrits dans `engine/README.md` ; `null_pad_codes.h` en est la copie côté firmware, vérifiée par un test (`engine/tests/firmware_codes.rs`).

## À tester sur le vrai pad

- [ ] **COL4 / P0.10** : M1–M3 et les deux clics répondent (broche NFC).
- [ ] **Pavé** : en profil 1, logiciel fermé, chaque touche tape le bon caractère ; la molette gauche règle le volume.
- [ ] **Codes** : avec le moteur (logs), une touche « M » et la molette droite produisent l'entrée et le profil attendus.
- [ ] **Crans** : un cran = un appui ; le volume bouge à chaque cran en USB **et** en Bluetooth (chaque cran est un appui de 0 ms) ; le sens horaire est bien `CW` ; sinon corriger `steps` / `triggers-per-rotation` ou inverser A/B.
- [ ] **Écran** : affiche `DEF`, puis `P02` / `P03` après M2 / M3.
- [ ] **Bluetooth** : le moteur reconnaît le pad (VID/PID `0x1D50`/`0x615E`).
- [ ] **Studio** : se connecte en USB après M1 + M3 ; une touche cachée de molette modifiée dans Studio change bien la molette.
- [ ] **Anti-rebond** : pas de double frappe avec 1 ms à l'appui ; sinon remonter `debounce-press-ms`.
