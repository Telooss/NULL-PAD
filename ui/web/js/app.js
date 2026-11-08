import { invoke } from './bridge.js';

const N = window.NullCtl;
const app = document.getElementById('app');
const stage = document.getElementById('stage');
const main = document.getElementById('main');

const cfg = await invoke('get_config');
await N.fontsReady();

const profiles = cfg.profiles.slice(0, 3).map((p) => ({ name: p.name, tag: p.tag }));
while (profiles.length < 3) profiles.push(null);
document.getElementById('shell').outerHTML = N.shell({ profiles, active: 0 });
stage.insertAdjacentHTML('beforeend', N.frame('0x4E50 // NODE 01 // NULL/PAD // ZMK', 'NP-01-DFLT-0001'));
document.getElementById('panel').innerHTML =
  '<div class="nc-panel-head"><div><div class="nc-label">Touche</div><div class="nc-keyname">—</div></div></div>' +
  '<div class="nc-section"><span class="nc-label">Choisis une touche</span></div>';

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
