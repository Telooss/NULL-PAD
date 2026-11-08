// Le pad 3D : enrobe NullCtl.pad3d (bundle.js) et parle en InputId. Rendu a la demande seulement.
import * as THREE from '../vendor/three.module.js';

// Pieces du modele -> entrees : F1-F20 = K00-K19, F21-F23 = M1-M3, D1/D2 = molettes (clic).
export function inputOf(piece) {
  const n = /^F(\d+)$/.exec(piece)?.[1];
  if (n) return +n <= 20 ? `K${String(n - 1).padStart(2, '0')}` : `M${n - 20}`;
  return { D1: 'ENC_L_PRESS', D2: 'ENC_R_PRESS' }[piece] ?? null;
}

// Inverse ; les rotations d'une molette designent la molette.
export function pieceOf(input) {
  const k = /^K(\d\d)$/.exec(input)?.[1];
  if (k && +k < 20) return `F${+k + 1}`;
  const m = /^M([123])$/.exec(input)?.[1];
  if (m) return `F${+m + 20}`;
  const e = /^ENC_([LR])_/.exec(input)?.[1];
  return e ? (e === 'L' ? 'D1' : 'D2') : null;
}

export function createPad(stageEl, geo, opts = {}) {
  let pickCb = opts.onPick ?? null;
  const p = window.NullCtl.pad3d(THREE, geo, stageEl, {
    fill: 0.9, cx: 0.5, ...opts,
    onPick: (piece) => pickCb?.(inputOf(piece)),
  });
  const ro = new ResizeObserver(() => p.resize(stageEl.clientWidth, stageEl.clientHeight));
  ro.observe(stageEl);
  const pulses = new Map();

  return {
    // map : InputId -> { text: [l1, l2], kind } ; une touche libre affiche le « + » gris.
    setLegends(map) {
      const arr = Array.from({ length: 23 }, (_, i) => {
        const l = map[inputOf(`F${i + 1}`)];
        return !l || l.kind === 'free' ? '' : l.text;
      });
      p.legends(arr);
    },
    setLcd: (L) => p.lcd(L),
    select: (input) => p.select(input && pieceOf(input)),
    hover: (input) => p.hover(input && pieceOf(input)),
    press(input, down) {
      const piece = pieceOf(input);
      if (!piece) return;
      if (!/^ENC_.*_C?CW$/.test(input)) return p.press(piece, down);
      // Rotation : courte lueur, un timer (pas de boucle).
      if (!down) return;
      clearTimeout(pulses.get(piece));
      p.press(piece, true, 0);
      pulses.set(piece, setTimeout(() => p.press(piece, false, 0), 150));
    },
    pickAt: (x, y) => inputOf(p.pick(x, y)),
    onPick: (cb) => { pickCb = cb; },
    frames: () => p.frames(),
    dispose() {
      ro.disconnect();
      pulses.forEach(clearTimeout);
      p.dispose();
    },
  };
}
