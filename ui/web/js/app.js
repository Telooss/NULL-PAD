import { invoke, listen, onFileDrop, pickFile, onCloseRequested } from './bridge.js';
import { createPad } from './pad.js';
import { mountShell } from './shell.js';
import { createViewPanel } from './viewpanel.js';
import { createEditPanel } from './editpanel.js';
import { startLive, toast } from './live.js';
import { legendsFor, panelModel, profileOf, profileSlots } from './view.js';
import { createDraft } from './draft.js';
import { actionForDrop, refusal } from './drop.js';
import { saveFlow, guard, confirmBox, modalOpen } from './config-mode.js';

const N = window.NullCtl;
const app = document.getElementById('app');
const stage = document.getElementById('stage');
const main = document.getElementById('main');
const panelEl = document.getElementById('panel');

let cfg = null;
let cfgError = null;
const [natives, geo] = await Promise.all([
  invoke('native_keys').catch(() => ({})), fetch('assets/nullpad.json').then((r) => r.json()), N.fontsReady(),
  invoke('get_config').then((c) => { cfg = c; }, (e) => { cfgError = String(e); }),
]);

const slots = profileSlots(cfg);
let profileId = slots.find(Boolean)?.id ?? 1;
let padProfile = profileId; // dernier profil annonce par le pad
let selected = null;
let link = 'none';
let draft = null; // non nul = mode configuration
let configWaiter = null;
let saving = false; // enregistrement en cours : boutons, profils, depots et panneau figes

const shell = mountShell(slots, profileId);
stage.insertAdjacentHTML('beforeend', N.frame('0x4E50 // NODE 01 // NULL/PAD // ZMK', 'NP-01-DFLT-0001'));
const banner = document.createElement('div');
banner.className = 'nc-configbar nc-label';
banner.textContent = 'CONFIG // NON ENREGISTRÉ';
banner.hidden = true;
stage.appendChild(banner);

const requestConfig = () => { if (!cfgError && !draft) enterConfig(); };
const viewPanel = createViewPanel(panelEl, { onConfigure: requestConfig });
const editPanel = createEditPanel(panelEl, { pickFile, onChange: () => refreshPad() });
shell.setLink('none');
shell.setEngine(false);
shell.onConfigure(requestConfig);
shell.setConfigurable(!cfgError);
if (cfgError) viewPanel.block(`Config illisible : ${cfgError} — corrigez le fichier ou réenregistrez depuis les réglages`);

const pad = createPad(stage, geo, { lcd: { top: '---', tag: '', bottom: '' } });

// Ce qui est affiche : le brouillon fusionne en configuration, la config sinon.
const shown = () => (draft ? draft.merged() : cfg);

function refreshPanel() {
  if (cfgError) return;
  if (draft) editPanel.show({ input: selected, draft, profileId, natives });
  else viewPanel.show(selected && { ...panelModel(cfg, profileId, selected, natives), input: selected });
  N.wire(main);
}

function refreshLcd() {
  const top = { usb: 'USB', ble: 'BLE' }[link] ?? '---';
  pad.setLcd({ top, tag: (profileOf(shown(), profileId)?.tag ?? '').toUpperCase(), bottom: `P${profileId}` });
}

function refreshPad() {
  pad.setLegends(legendsFor(shown(), profileId, natives));
  refreshLcd();
}

function setProfile(id, glitch) {
  if (!profileOf(cfg, id) || (id === profileId && glitch)) return;
  profileId = id;
  shell.setActive(id);
  refreshPad();
  refreshPanel();
  if (glitch) N.glitch(app);
}

// Profil annonce par le pad : suivi en mode vue ; en configuration, l'affichage reste sur le brouillon.
function followPad(id, glitch) {
  padProfile = id;
  if (!draft) return setProfile(id, glitch);
  if (id !== profileId && glitch) toast(app, `Le pad est passé sur ${profileOf(cfg, id)?.tag ?? `P${id}`}`);
}

function setLink(l) {
  link = l;
  shell.setLink(l);
  refreshLcd();
}

function select(input) {
  selected = input;
  pad.select(input);
  refreshPanel();
}

// Config relue sur le disque (apres un `config` ok ou notre propre enregistrement).
async function reload() {
  try {
    cfg = await invoke('get_config');
  } catch {
    return;
  }
  if (!profileOf(cfg, profileId)) return;
  refreshPad();
  if (!draft) refreshPanel();
}

// ── Mode configuration ─────────────────────────────────────
function enterConfig() {
  draft = createDraft(cfg, profileId);
  stage.classList.add('is-config');
  banner.hidden = false;
  shell.setConfigMode(true);
  editPanel.setError(null);
  refreshPad();
  refreshPanel();
}

// follow : revenir au profil du pad (pas quand on quitte pour changer de profil affiche).
function exitConfig(follow = true) {
  draft = null;
  stage.classList.remove('is-config');
  banner.hidden = true;
  shell.setConfigMode(false);
  refreshPad();
  refreshPanel();
  if (follow && padProfile !== profileId) setProfile(padProfile, true);
}

// -> { promise, cancel } ; le minuteur ne retire que sa propre attente.
function waitConfigEvent(ms) {
  let mine;
  const promise = new Promise((resolve) => {
    const t = setTimeout(() => { if (configWaiter === mine) configWaiter = null; resolve(null); }, ms);
    mine = (e) => { clearTimeout(t); if (configWaiter === mine) configWaiter = null; resolve(e); };
  });
  configWaiter = mine;
  return { promise, cancel: () => mine(null) };
}

// Enregistre le brouillon ; true si c'est fait (et le mode quitte), false si le brouillon reste.
async function commit(follow = true) {
  if (!draft || saving) return false;
  saving = true;
  panelEl.inert = true;
  editPanel.setError(null);
  const merged = draft.merged();
  let r;
  try {
    r = await saveFlow({
      json: JSON.stringify(merged),
      validate: (json) => invoke('validate_config', { json }),
      save: (json) => invoke('save_config', { json }),
      waitConfigEvent,
      engineUp: shell.engineUp,
      discard: () => { cfg = merged; exitConfig(follow); },
    });
  } finally {
    saving = false;
    panelEl.inert = false;
  }
  if (r.state === 'invalid' || r.state === 'rejected') {
    const msg = r.state === 'rejected' ? `Refusé par le moteur (brouillon gardé) : ${r.error}` : r.write ? `Écriture impossible : ${r.error}` : r.error;
    editPanel.setError(msg);
    toast(app, r.state === 'rejected' ? 'Refusé par le moteur' : r.write ? 'Écriture impossible : rien n’a changé' : 'Config invalide : rien n’a été écrit');
    return false;
  }
  toast(app, r.state === 'saved' ? 'Enregistré' : 'Enregistré, pris en compte au démarrage du moteur');
  shell.setSaved(true);
  N.glitch(app);
  reload();
  return true;
}

const LEAVE = { title: 'Changements non enregistrés', text: 'Le brouillon de ce profil a des changements non enregistrés.',
  buttons: [['save', 'Enregistrer', true], ['discard', 'Abandonner'], ['stay', 'Rester']] };

// Quitter le brouillon (changer de profil, fermer) : true si on peut partir.
async function leaveDraft(follow) {
  if (guard(draft.dirty(), 'leave') === 'proceed') { exitConfig(follow); return true; }
  const c = await confirmBox(app, LEAVE);
  if (!draft) return true; // mode deja quitte pendant la question
  if (c === 'save') return commit(follow);
  if (c === 'discard') { exitConfig(follow); return true; }
  return false;
}

shell.onSave(() => { if (draft) commit(); });
shell.onCancel(async () => {
  if (!draft || saving) return;
  if (guard(draft.dirty(), 'cancel') === 'ask') {
    const c = await confirmBox(app, { title: 'Annuler la configuration', text: 'Les changements non enregistrés seront perdus.',
      buttons: [['discard', 'Abandonner les changements'], ['stay', 'Rester', true]] });
    if (c !== 'discard') return;
  }
  exitConfig();
});
shell.onSlot(async (id) => {
  if (!draft) return setProfile(id, true);
  if (id === profileId || !profileOf(cfg, id)) return;
  if (saving || !(await leaveDraft(false))) return;
  setProfile(id, true);
  enterConfig(); // la configuration continue sur le profil choisi, avec un nouveau brouillon
});
onCloseRequested(() => (!draft || guard(draft.dirty(), 'close') === 'proceed' ? true : leaveDraft(false)));

refreshPad();
pad.onPick((input) => { if (input && !saving) select(input); });

// Depot de fichiers (natif Tauri, ou mock) ecoute au niveau de la webview / du document.
onFileDrop(({ type, paths, x, y }) => {
  const busy = saving || modalOpen(); // pendant un enregistrement ou une question : depot ignore
  const target = type === 'leave' || busy ? null : pad.pickAt(x, y);
  // Lueur seulement sur une cible liable (pas de natives, M1–M3, molette gauche).
  if (type !== 'drop') { if (draft) pad.hover(target && !refusal(target, profileId, natives) ? target : null); return; }
  pad.hover(null);
  if (busy) return;
  if (!draft) return toast(app, 'Passer en configuration pour modifier');
  if (!target) return toast(app, 'Déposer le fichier sur une touche ou une molette');
  const out = actionForDrop(paths, target, profileId, natives, draft.sounds().map((s) => s.id));
  if (out.error) return toast(app, out.error);
  if (out.sound) draft.addSound(out.sound);
  draft.set(target, 'Press', { action: out.action });
  editPanel.showTrigger(target, 0); // l'action deposee est sur l'appui court
  refreshPad();
  select(target);
  if (out.warning) toast(app, out.warning);
});

const q = new URLSearchParams(location.search);
if (q.get('test') === '1') {
  window.__padFrames = () => pad.frames();
  window.__layoutOk = () => document.querySelector('.nc-topbar').scrollWidth <= innerWidth && document.querySelector('.nc-panel').getBoundingClientRect().right <= innerWidth &&
    (innerWidth < 1200 || [...document.querySelectorAll('.nc-sticker')].every((s) => s.scrollWidth <= s.clientWidth)); // ellipse admise pres du minimum
}

N.icons();
N.wire(main);

await startLive({
  invoke, listen, app, pad, shell, setProfile: followPad, setLink,
  // L'evenement `action` ne porte pas de profil : celui du pad, pas celui affiche.
  binding: (input, trigger, p) => profileOf(cfg, p ?? padProfile)?.bindings.find((b) => b.input === input && b.trigger === trigger),
  onConfig: (e) => {
    if (configWaiter) configWaiter(e);
    else if (e.ok) reload();
  },
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
