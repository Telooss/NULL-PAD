#!/usr/bin/env bash
# Compile le firmware NULL/PAD en local (à lancer dans WSL Ubuntu).
# Depuis Windows : wsl -e bash firmware/build-local.sh   (à la racine du dépôt)
set -euo pipefail

# Toolchain installée une fois (voir firmware/README.md)
export ZEPHYR_TOOLCHAIN_VARIANT=zephyr
export ZEPHYR_SDK_INSTALL_DIR="$HOME/zephyr-sdk-0.16.8"
source "$HOME/zmk-venv/bin/activate"

SRC="$(cd "$(dirname "$0")/.." && pwd)"
WS="$HOME/np/repo"
mkdir -p "$WS"

# Copie du dépôt. Pas de --delete global : zmk/, zephyr/ et modules/ appartiennent à west
# (le zephyr/module.yml du dépôt est simplement déposé dans le dossier zephyr de west).
rsync -a --exclude target --exclude .git --exclude .superpowers --exclude .west   --exclude /zmk --exclude /modules --exclude /build --exclude firmware/out "$SRC/" "$WS/"
rsync -a --delete --exclude out "$SRC/firmware/" "$WS/firmware/"  # propage les suppressions

cd "$WS"
[ -d .west ] || west init -l firmware
west update --narrow -o=--depth=1
west zephyr-export

west build -p -s zmk/app -d build -b nice_nano_v2 -S studio-rpc-usb-uart -- \
  -DZMK_CONFIG="$HOME/np/repo/firmware" \
  "-DSHIELD=null_pad nice_view_adapter nice_view" \
  -DZMK_EXTRA_MODULES="$HOME/np/repo" \
  -DCONFIG_ZMK_STUDIO=y

mkdir -p "$SRC/firmware/out"
cp build/zephyr/zmk.uf2 "$SRC/firmware/out/null_pad.uf2"
ls -l "$SRC/firmware/out/null_pad.uf2"
