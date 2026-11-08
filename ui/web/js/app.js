import { invoke, listen } from './bridge.js';
import { createPad, inputOf } from './pad.js';
import { legendFor, fitLegend } from './legend.js';

const N = window.NullCtl;
const app = document.getElementById('app');
const stage = document.getElementById('stage');
const main = document.getElementById('main');

const [cfg, natives, geo] = await Promise.all([
  invoke('get_config'), invoke('native_keys'), fetch('assets/nullpad.json').then((r) => r.json()), N.fontsReady(),
]);

const profiles = cfg.profiles.slice(0, 3).map((p) => ({ name: p.name, tag: p.tag }));
while (profiles.length < 3) profiles.push(null);
document.getElementById('shell').outerHTML = N.shell({ profiles, active: 0 });
stage.insertAdjacentHTML('beforeend', N.frame('0x4E50 // NODE 01 // NULL/PAD // ZMK', 'NP-01-DFLT-0001'));
document.getElementById('panel').innerHTML =
  '<div class="nc-panel-head"><div><div class="nc-label">Touche</div><div class="nc-keyname">—</div></div></div>' +
  '<div class="nc-section"><span class="nc-label">Choisis une touche</span></div>';

// Le pad 3D, legendes du profil 1.
const pad = createPad(stage, geo, { lcd: { top: 'USB', tag: 'DEF', bottom: 'P1' } });
const prof = cfg.profiles[0];
const legends = {};
for (let i = 1; i <= 23; i++) {
  const input = inputOf(`F${i}`);
  const l = legendFor(prof?.bindings.find((b) => b.input === input), natives, cfg.sound?.sounds, input);
  legends[input] = { text: fitLegend(l.text), kind: l.kind };
}
pad.setLegends(legends);
pad.onPick((input) => { pad.select(input); N.wire(main); });

const q = new URLSearchParams(location.search);
if (q.get('test') === '1') window.__padFrames = () => pad.frames();
if (q.get('demo') === '1') {
  listen('engine-event', ({ payload: e }) => {
    if (e.type !== 'key') return;
    pad.press(e.input, true);
    setTimeout(() => pad.press(e.input, false), 120);
  });
}

N.icons();
N.wire(main);
document.querySelectorAll('.nc-slot').forEach((s) => s.addEventListener('click', () => N.glitch(app)));

// Boutons de fenetre : seulement sous Tauri (voir capabilities/default.json).
const W = window.__TAURI__?.window?.getCurrentWindow?.();
if (W) {
  const tb = document.querySelector('.nc-titlebar');
  tb.setAttribute('data-tauri-drag-region', '');
  const [min, max, close] = tb.querySelectorAll('.nc-winctl span');
  min.onclick = () => W.minimize();
  max.onclick = () => W.toggleMaximize();
  close.onclick = () => W.close();
}
