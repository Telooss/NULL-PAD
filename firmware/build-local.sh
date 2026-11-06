#!/usr/bin/env bash
# Compile le firmware NULL/PAD en local (à lancer dans WSL Ubuntu).
# Depuis Windows : wsl -e bash firmware/build-local.sh   (à la racine du dépôt)
set -euo pipefail

# Toolchain installée une fois (voir firmware/README.md)
for d in "$HOME/zmk-venv" "$HOME/zephyr-sdk-0.16.8"; do
  [ -d "$d" ] || { echo "Manque $d : lancer d'abord l'installation unique de firmware/README.md" >&2; exit 1; }
done
export ZEPHYR_TOOLCHAIN_VARIANT=zephyr
export ZEPHYR_SDK_INSTALL_DIR="$HOME/zephyr-sdk-0.16.8"
source "$HOME/zmk-venv/bin/activate"

REPO="$(cd "$(dirname "$0")/.." && pwd)"
SRC="$HOME/np/src"   # copie du dépôt (module ZMK_EXTRA_MODULES)
WS="$HOME/np/ws"     # espace west : firmware/ + zmk, zephyr, modules (gérés par west)
mkdir -p "$SRC" "$WS/firmware"

rsync -a --delete --exclude /target --exclude /.git --exclude /.superpowers --exclude /firmware/out "$REPO/" "$SRC/"
rsync -a --delete --exclude /out "$REPO/firmware/" "$WS/firmware/"

cd "$WS"
[ -d .west ] || west init -l firmware
# HTTP/1.1 + reprises : le HTTP/2 de GitHub coupe parfois les gros fetchs
export GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=http.version GIT_CONFIG_VALUE_0=HTTP/1.1
for i in 1 2 3 4 5; do west update --narrow -o=--depth=1 && break || [ "$i" -lt 5 ]; done
west zephyr-export
pip install -q -r zephyr/scripts/requirements-base.txt

west build -p -s zmk/app -d build -b nice_nano_v2 -S studio-rpc-usb-uart -- \
  -DZMK_CONFIG="$WS/firmware" \
  "-DSHIELD=null_pad nice_view_adapter nice_view" \
  -DZMK_EXTRA_MODULES="$SRC" \
  -DCONFIG_ZMK_STUDIO=y

mkdir -p "$REPO/firmware/out"
cp build/zephyr/zmk.uf2 "$REPO/firmware/out/null_pad.uf2"
ls -l "$REPO/firmware/out/null_pad.uf2"
