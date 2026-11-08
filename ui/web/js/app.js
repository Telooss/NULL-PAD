import { invoke, listen } from './bridge.js';
import { createPad } from './pad.js';
import { mountShell } from './shell.js';
import { createViewPanel } from './viewpanel.js';
import { startLive, toast } from './live.js';
import { legendsFor, panelModel, profileOf, profileSlots } from './view.js';

const N = window.NullCtl;
const app = document.getElementById('app');
const stage = document.getElementById('stage');
const main = document.getElementById('main');

let cfg = null;
let cfgError = null;
const [natives, geo] = await Promise.all([
  invoke('native_keys').catch(() => ({})), fetch('assets/nullpad.json').then((r) => r.json()), N.fontsReady(),
  invoke('get_config').then((c) => { cfg = c; }, (e) => { cfgError = String(e); }),
]);

const slots = profileSlots(cfg);
let profileId = slots.find(Boolean)?.id ?? 1;
let selected = null;
let link = 'none';

const shell = mountShell(slots, profileId);
stage.insertAdjacentHTML('beforeend', N.frame('0x4E50 // NODE 01 // NULL/PAD // ZMK', 'NP-01-DFLT-0001'));
const panel = createViewPanel(document.getElementById('panel'), {
  onConfigure: () => document.dispatchEvent(new CustomEvent('request-config-mode', { detail: { profileId } })),
});
shell.setLink('none');
shell.setEngine(false);
shell.onConfigure(() => !cfgError && document.dispatchEvent(new CustomEvent('request-config-mode', { detail: { profileId } })));
shell.setConfigurable(!cfgError);
if (cfgError) panel.block(`Config illisible : ${cfgError} — corrigez le fichier ou réenregistrez depuis les réglages`);

const pad = createPad(stage, geo, { lcd: { top: '---', tag: '', bottom: '' } });

function refreshPanel() {
  if (cfgError) return;
  panel.show(selected && { ...panelModel(cfg, profileId, selected, natives), input: selected });
  N.wire(main);
}

function refreshLcd() {
  const top = { usb: 'USB', ble: 'BLE' }[link] ?? '---';
  pad.setLcd({ top, tag: (profileOf(cfg, profileId)?.tag ?? '').toUpperCase(), bottom: `P${profileId}` });
}

function setProfile(id, glitch) {
  if (!profileOf(cfg, id) || (id === profileId && glitch)) return;
  profileId = id;
  shell.setActive(id);
  pad.setLegends(legendsFor(cfg, id, natives));
  refreshLcd();
  refreshPanel();
  if (glitch) N.glitch(app);
}

function setLink(l) {
  link = l;
  shell.setLink(l);
  refreshLcd();
}

pad.setLegends(legendsFor(cfg, profileId, natives));
refreshLcd();
pad.onPick((input) => {
  if (!input) return;
  selected = input;
  pad.select(input);
  refreshPanel();
});
shell.onSlot((id) => setProfile(id, true));

// Depot de fichier : refuse en mode vue (le depot natif de Tauri arrive avec le mode configuration).
stage.addEventListener('dragover', (e) => e.preventDefault());
stage.addEventListener('drop', (e) => {
  e.preventDefault();
  toast(app, 'Passer en configuration pour modifier');
});

const q = new URLSearchParams(location.search);
if (q.get('test') === '1') window.__padFrames = () => pad.frames();

N.icons();
N.wire(main);

await startLive({
  invoke, listen, app, pad, shell, setProfile, setLink,
  binding: (input, trigger) => profileOf(cfg, profileId)?.bindings.find((b) => b.input === input && b.trigger === trigger),
});

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
