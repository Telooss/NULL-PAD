import { invoke, listen, onFileDrop, pickFile, onCloseRequested } from './bridge.js';
import { createPad } from './pad.js';
import { mountShell } from './shell.js';
import { createViewPanel } from './viewpanel.js';
import { createEditPanel } from './editpanel.js';
import { startLive, toast } from './live.js';
import { legendsFor, panelModel, profileOf, profileSlots } from './view.js';
import { createDraft, mergeDraft } from './draft.js';
import { actionForDrop, refusal } from './drop.js';
import { saveFlow, guard, confirmBox, modalOpen } from './config-mode.js';
import { createSettings } from './settings.js';
import { initFx } from './fx.js';
import { barSlots, nextFreeId, withNewProfile, withoutProfile, openCreate, openMenu, cleanTag, slotLabel, NAME_MAX } from './profiles.js';
import { createPadFlow } from './pad-flow.js';
import { isAction, setNative, setAction, renameLayer, withLayer, copyLayer, samePad, withoutOrphans, padErrorText } from './pad-model.js';

const N = window.NullCtl;
const app = document.getElementById('app');
const stage = document.getElementById('stage');
const main = document.getElementById('main');
const panelEl = document.getElementById('panel');

let cfg = null;
let cfgError = null;
const [natives, geo, , , catalog] = await Promise.all([
  invoke('native_keys').catch(() => ({})), fetch('assets/nullpad.json').then((r) => r.json()), N.fontsReady(),
  invoke('get_config').then((c) => { cfg = c; }, (e) => { cfgError = String(e); }),
  invoke('hid_catalog').catch(() => []),
]);

const slots = profileSlots(cfg);
let profileId = slots.find(Boolean)?.id ?? 1;
let padProfile = profileId; // dernier profil annonce par le pad
let selected = null;
let link = 'none';
let draft = null; // non nul = mode configuration
let configWaiter = null;
let saving = false; // enregistrement en cours : boutons, profils, depots et panneau figes
let padEdit = null; // pad modifiable dans cette configuration : { expected (lecture), target (brouillon du pad), maxLen }
let entering = false; // lecture du pad avant le mode configuration

const shell = mountShell();
const topbar = document.querySelector('.nc-topbar');
let creating = false; // ecran de creation ouvert : barre des profils et mode configuration figes
stage.insertAdjacentHTML('beforeend', N.frame('0x4E50 // NODE 01 // NULL/PAD // ZMK', 'NP-01-DFLT-0001'));
const banner = document.createElement('div');
banner.className = 'nc-configbar nc-label';
banner.textContent = 'CONFIG // NON ENREGISTRÉ';
banner.hidden = true;
stage.appendChild(banner);
// Tete de la scene en configuration : nom et tag du profil (editent le brouillon).
const rename = document.createElement('div');
rename.className = 'nc-rename';
rename.hidden = true;
rename.innerHTML = `<label class="nc-label">Nom<input class="nc-input" data-name maxlength="${NAME_MAX}" spellcheck="false" autocomplete="off"></label>` +
  '<label class="nc-label">Tag<input class="nc-input" data-tag maxlength="3" spellcheck="false" autocomplete="off" aria-describedby="nc-tag-hint"></label>' +
  '<span class="nc-label" id="nc-tag-hint">A–Z 0–9 · 3 caractères</span>';
stage.appendChild(rename);
const nameIn = rename.querySelector('[data-name]');
const tagIn = rename.querySelector('[data-tag]');

// Barre des profils : relue depuis ce qui est affiche (le brouillon fusionne montre les renommages en cours).
function renderBar() {
  shell.setSlots(barSlots(shown()), profileId);
  shell.setCanCreate(!cfgError && nextFreeId(cfg) !== null);
}

// Entree en configuration : le pad branche est lu d'abord (M1 + M3 s'il est verrouille) ; sinon natives en lecture seule.
const requestConfig = async () => {
  if (cfgError || draft || creating || entering || saving || settings.isOpen()) return;
  entering = true;
  topbar.inert = true;
  try {
    padEdit = await padFlow.edit();
  } finally {
    entering = false;
    topbar.inert = false;
  }
  if (!draft && !creating && !settings.isOpen()) enterConfig();
};
const viewPanel = createViewPanel(panelEl, { onConfigure: requestConfig });
const editPanel = createEditPanel(panelEl, { pickFile, onChange: () => refreshPad() });
shell.setLink('none');
shell.setEngine(false);
shell.onConfigure(requestConfig);
shell.setConfigurable(!cfgError);
if (cfgError) viewPanel.block(`Config illisible : ${cfgError} — corrigez le fichier ou réenregistrez depuis les réglages`);

const pad = createPad(stage, geo, { lcd: { top: '---', tag: '', bottom: '' } });
const padFlow = createPadFlow({ invoke, listen, app, shell, toast: (t) => toast(app, t), onModel: () => { refreshPad(); if (!draft) refreshPanel(); } });

// Keymap du pad pour les legendes et les regles : la cible modifiable en configuration, sinon la derniere connue (lecture seule).
const padCtx = () => (draft && padEdit ? { model: padEdit.target, editable: true } : padFlow.model() ? { model: padFlow.model(), editable: false } : null);
const padUi = () => { const c = padCtx(); return c && { ...c, catalog, toNative, toAction }; };
// Passer en native retire les liaisons nullctl de l'entree ; repasser en action remet Action{p, i}.
function toNative(input, b) { padEdit.target = setNative(padEdit.target, draft, profileId, input, b); refreshPad(); refreshPanel(); }
function toAction(input) { padEdit.target = setAction(padEdit.target, profileId, input); refreshPad(); refreshPanel(); }
const dirty = () => draft.dirty() || (!!padEdit && !samePad(padEdit.expected, padEdit.target));
// Validation et ecriture de la config contre la keymap ecrite, ou la derniere connue (sinon la table du firmware).
// Config ecrite contre la keymap `model` (ou la derniere connue) : les actions sur des touches natives dans le pad
// ne peuvent plus se declencher et feraient refuser toute la config ; retirees, avec un toast (sauf quiet).
function dropOrphans(next, model = padFlow.model(), quiet = false) {
  if (!model) return next;
  const { config, removed } = withoutOrphans(next, model);
  // Apres le toast de l'enregistrement (qui le remplacerait sinon).
  if (removed.length && !quiet) setTimeout(() => toast(app, `Actions retirées (touches natives dans le pad) : ${removed.map((x) => `P${x.profile} ${x.input}`).join(', ')}`), 1500);
  return config;
}
const cfgCalls = {
  validate: (json, p) => invoke('validate_config', { json, pad: p ?? padFlow.model() }),
  save: (json, p) => invoke('save_config', { json, pad: p ?? padFlow.model() }),
};

// Ce qui est affiche : le brouillon fusionne en configuration, la config sinon.
const shown = () => (draft ? draft.merged() : cfg);
renderBar();

function refreshPanel() {
  if (cfgError) return;
  if (draft) editPanel.show({ input: selected, draft, profileId, natives, pad: padUi() });
  else viewPanel.show(selected && { ...panelModel(cfg, profileId, selected, natives, padCtx()?.model), input: selected });
  N.wire(main);
}

function refreshLcd() {
  const top = { usb: 'USB', ble: 'BLE' }[link] ?? '---';
  pad.setLcd({ top, tag: (profileOf(shown(), profileId)?.tag ?? '').toUpperCase(), bottom: `P${profileId}` });
}

function refreshPad() {
  pad.setLegends(legendsFor(shown(), profileId, natives, padCtx()?.model));
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
  if (!profileOf(cfg, profileId)) profileId = 1;
  renderBar();
  refreshPad();
  if (!draft) refreshPanel();
}

// ── Mode configuration ─────────────────────────────────────
function enterConfig() {
  if (padEdit) padEdit.target = structuredClone(padEdit.expected); // changements du pad d'un brouillon abandonne
  draft = createDraft(cfg, profileId);
  stage.classList.add('is-config');
  banner.hidden = false;
  shell.setConfigMode(true);
  editPanel.setError(null);
  nameIn.value = draft.get().name;
  tagIn.value = draft.get().tag;
  tagIn.removeAttribute('aria-invalid');
  rename.hidden = false;
  renderBar();
  refreshPad();
  refreshPanel();
}

// follow : revenir au profil du pad (pas quand on quitte pour changer de profil affiche).
function exitConfig(follow = true) {
  draft = null;
  stage.classList.remove('is-config');
  banner.hidden = true;
  rename.hidden = true;
  shell.setConfigMode(false);
  renderBar();
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
  rename.inert = true;
  editPanel.setError(null);
  let merged;
  let r;
  try {
    // Relue juste avant : les reglages ou autres profils modifies entre-temps sont gardes.
    try {
      merged = dropOrphans(mergeDraft(await invoke('get_config'), draft.get(), draft.soundChanges()), padEdit?.target);
    } catch (e) {
      toast(app, `Config illisible, rien n’a été écrit : ${e?.message ?? e}`);
      return false;
    }
    r = await saveFlow({
      json: JSON.stringify(merged),
      ...cfgCalls,
      waitConfigEvent,
      engineUp: shell.engineUp,
      discard: () => { cfg = merged; exitConfig(follow); },
      pad: padEdit && padFlow.deps(padEdit.expected, padEdit.target),
    });
  } finally {
    saving = false;
    panelEl.inert = false;
    rename.inert = false;
  }
  // Pad ecrit : c'est la nouvelle lecture de reference, meme si la config echoue ensuite.
  padFlow.settle(r);
  if (r.padModel && padEdit) padEdit = { ...padEdit, expected: r.padModel, target: structuredClone(r.padModel) };
  if (r.state === 'pad') {
    editPanel.setError(`Pad non écrit, brouillon gardé : ${r.error}`);
    const re = await padFlow.onPadError(r);
    if (re && draft) {
      padEdit = { expected: re.model, target: structuredClone(re.model), maxLen: re.max_name_len };
      if (draft.get().tag !== profileOf(cfg, profileId)?.tag) padEdit.target = renameLayer(padEdit.target, profileId, draft.get().tag, padEdit.maxLen);
      // Les actions posees sur des touches redevenues natives ne pourraient plus s'enregistrer.
      const lost = draft.get().bindings.filter((b) => !isAction(re.model, profileId, b.input));
      for (const b of lost) draft.set(b.input, b.trigger, null);
      toast(app, `Pad relu : ses changements sont perdus${lost.length ? ', avec les actions des touches natives' : ''}. Le reste du brouillon est gardé`);
      refreshPad();
      refreshPanel();
    }
    return false;
  }
  if (r.state === 'invalid' || r.state === 'rejected') {
    const msg = r.state === 'rejected' ? `Refusé par le moteur (brouillon gardé) : ${r.error}` : r.write ? `Écriture impossible : ${r.error}` : r.error;
    const padFirst = r.padUpdated && r.state === 'invalid';
    editPanel.setError(padFirst ? `Pad mis à jour, config non enregistrée : ${msg}` : msg);
    toast(app, padFirst ? 'Pad mis à jour, config non enregistrée' : r.state === 'rejected' ? 'Refusé par le moteur' : r.write ? 'Écriture impossible : rien n’a changé' : 'Config invalide : rien n’a été écrit');
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
  if (guard(dirty(), 'leave') === 'proceed') { exitConfig(follow); return true; }
  const c = await confirmBox(app, LEAVE);
  if (!draft) return true; // mode deja quitte pendant la question
  if (c === 'save') return commit(follow);
  if (c === 'discard') { exitConfig(follow); return true; }
  return false;
}

shell.onSave(() => { if (draft) commit(); });
shell.onCancel(async () => {
  if (!draft || saving) return;
  if (guard(dirty(), 'cancel') === 'ask') {
    const c = await confirmBox(app, { title: 'Annuler la configuration', text: 'Les changements non enregistrés seront perdus.',
      buttons: [['discard', 'Abandonner les changements'], ['stay', 'Rester', true]] });
    if (c !== 'discard') return;
  }
  exitConfig();
});
shell.onSlot(async (id) => {
  if (saving || creating || settings.isOpen()) return;
  if (!draft) return setProfile(id, true);
  if (id === profileId || !profileOf(cfg, id)) return;
  if (saving || !(await leaveDraft(false))) return;
  setProfile(id, true);
  enterConfig(); // la configuration continue sur le profil choisi, avec un nouveau brouillon
});
// Renommer (configuration seulement) : le brouillon ne recoit que des valeurs valides, les champs se corrigent a la sortie.
function applyMeta() {
  if (!draft) return;
  const cur = draft.get();
  const before = cur.tag;
  const tag = cleanTag(tagIn.value);
  if (tagIn.value !== tag) tagIn.value = tag;
  tagIn.toggleAttribute('aria-invalid', tag.length !== 3);
  draft.setProfileMeta({ name: nameIn.value.trim() || cur.name, tag: tag.length === 3 ? tag : cur.tag });
  if (padEdit && cur.tag !== before) padEdit.target = renameLayer(padEdit.target, profileId, cur.tag, padEdit.maxLen); // l'ecran du pad affiche le tag
  renderBar();
  refreshLcd();
}
nameIn.addEventListener('input', applyMeta);
tagIn.addEventListener('input', applyMeta);
nameIn.addEventListener('blur', () => { if (draft) nameIn.value = draft.get().name; });
tagIn.addEventListener('blur', () => { if (draft) { tagIn.value = draft.get().tag; tagIn.removeAttribute('aria-invalid'); } });

// ── Creer / supprimer : ecriture immediate (validation, enregistrement, evenement du moteur), jamais avec un brouillon modifie ──
// -> true si ecrit ; sinon un toast dit pourquoi et rien n'a change.
// layer (profils) : { id, bindings(model) -> PadBinding[], name } : la couche est ecrite dans le pad d'abord.
// Config validee avant de toucher au pad ; pad absent -> config seule et « Pad pas à jour » ; pad refuse ou annule -> rien.
async function writeNow(next, layer = null) {
  if (saving) return false;
  saving = true;
  panelEl.inert = true;
  let r;
  let padW = null;
  try {
    if (layer) {
      try {
        await cfgCalls.validate(JSON.stringify(dropOrphans(next, undefined, true)));
      } catch (e) {
        toast(app, `Config invalide : ${e}`);
        return false;
      }
      padW = await padLayer(layer);
      if (padW === false) return false;
    }
    next = dropOrphans(next, padW?.target);
    r = await saveFlow({
      json: JSON.stringify(next),
      ...cfgCalls,
      waitConfigEvent,
      engineUp: shell.engineUp,
      pad: padW && padFlow.deps(padW.expected, padW.target),
    });
  } finally {
    saving = false;
    panelEl.inert = false;
  }
  padFlow.settle(r);
  if (r.state === 'pad') {
    toast(app, padErrorText(r));
    return false;
  }
  if (layer && !padW) padFlow.markStale(layer.id);
  if (r.state === 'invalid' || r.state === 'rejected') {
    const why = r.state === 'rejected' ? `Refusé par le moteur : ${r.error}` : r.write ? `Écriture impossible : ${r.error}` : `Config invalide : ${r.error}`;
    toast(app, r.padUpdated && r.state === 'invalid' ? `Pad mis à jour, config non enregistrée : ${r.error}` : why);
    return false;
  }
  try {
    cfg = await invoke('get_config');
  } catch {
    cfg = next; // ecrit mais relecture impossible : on continue sur ce qui vient d'etre ecrit
    toast(app, 'Enregistré, mais la config n’a pas pu être relue');
  }
  shell.setSaved(true);
  return true;
}

// ── Reglages : remplace scene et panneau ; profils et mode configuration figes pendant ce temps ──
const settings = createSettings({
  main, app, invoke, pickFile, toast: (t) => toast(app, t), config: () => cfg, write: writeNow,
  onClose: () => { stage.style.display = ''; panelEl.style.display = ''; },
});
shell.onSettings(() => {
  if (cfgError) return toast(app, 'Config illisible : corrigez le fichier d’abord');
  if (draft) return toast(app, 'Quittez d’abord la configuration du profil');
  if (saving || creating || modalOpen() || settings.isOpen()) return;
  stage.style.display = 'none';
  panelEl.style.display = 'none';
  settings.open();
});

// Couche du pad pour une operation sur les profils (lecture avec M1 + M3 si besoin) -> { expected, target } ;
// null : pas de pad (config seule) ; false : pad occupe, refuse ou annule (toast affiche, on n'ecrit rien).
async function padLayer({ id, bindings, name }) {
  const e = await padFlow.edit('rien n’a été modifié', 'Annuler');
  if (!e) return e;
  if (!e.expected.layers[id - 1]) return null;
  try {
    return { expected: e.expected, target: withLayer(e.expected, id, await bindings(e.expected), name.slice(0, e.maxLen)) };
  } catch (err) {
    toast(app, `Pad non lu (${err}) : rien n’a été modifié`);
    return false;
  }
}

async function newProfile(want) {
  if (saving || creating || settings.isOpen() || modalOpen() || cfgError || nextFreeId(cfg) === null) return;
  if (draft && !(await leaveDraft(false))) return;
  const id = withNewProfile(cfg, { name: '', tag: 'XXX', want }).profiles.find((p) => !profileOf(cfg, p.id)).id;
  const fromId = profileId;
  const from = profileOf(cfg, fromId);
  const preview = ({ tag, copy }) => {
    pad.setLcd({ top: 'NEW', tag, bottom: `P${id}` });
    pad.setLegends(legendsFor(withNewProfile(cfg, { name: '', tag, copyFrom: copy ? fromId : undefined, want: id }), id, natives));
  };
  creating = true;
  topbar.inert = true;
  const out = await openCreate(main, { id, from, onPreview: preview });
  topbar.inert = false;
  creating = false;
  refreshPad();
  if (!out || draft) return;
  const next = withNewProfile(cfg, { name: out.name, tag: out.tag, copyFrom: out.copy ? fromId : undefined, want: id });
  const newId = next.profiles.find((p) => !profileOf(cfg, p.id)).id;
  const bindings = (m) => (out.copy ? copyLayer(m.layers[fromId - 1].bindings, fromId, newId) : invoke('pad_default_layer', { profile: newId }));
  if (!(await writeNow(next, { id: newId, bindings, name: out.tag }))) return;
  profileId = newId;
  renderBar();
  refreshPad();
  refreshPanel();
  N.glitch(app);
  toast(app, `Profil ${out.name} créé · ${slotLabel(newId)} → ${out.tag}`);
}

async function deleteProfile(id) {
  const p = profileOf(cfg, id);
  if (!p || id === 1 || saving || creating || settings.isOpen() || modalOpen()) return;
  if (draft && !(await leaveDraft(false))) return;
  const c = await confirmBox(app, { title: 'Supprimer le profil', text: `Supprimer le profil ${p.name} (${p.tag}) ? Ses actions seront perdues.`,
    buttons: [['delete', 'Supprimer'], ['stay', 'Annuler', true]] });
  const reset = { id, bindings: () => invoke('pad_default_layer', { profile: id }), name: `P0${id}` };
  if (c !== 'delete' || draft || creating || !(await writeNow(withoutProfile(cfg, id), reset))) return;
  if (!profileOf(cfg, profileId)) profileId = 1;
  if (!profileOf(cfg, padProfile)) padProfile = 1;
  renderBar();
  refreshPad();
  refreshPanel();
  N.glitch(app);
  toast(app, `Profil ${p.name} supprimé`);
}

shell.onNew(newProfile);
shell.onMenu((id, anchor) => !creating && !settings.isOpen() && openMenu(anchor, [['Supprimer le profil…', () => deleteProfile(id)]]));
onCloseRequested(() => (settings.isOpen() ? settings.leave() : !draft || guard(dirty(), 'close') === 'proceed' ? true : leaveDraft(false)));

refreshPad();
padFlow.init();
pad.onPick((input) => { if (input && !saving) select(input); });

// Depot de fichiers (natif Tauri, ou mock) ecoute au niveau de la webview / du document.
onFileDrop(({ type, paths, x, y }) => {
  if (settings.isOpen()) { if (type === 'drop' && !modalOpen()) settings.drop(paths); return; }
  const busy = saving || modalOpen(); // pendant un enregistrement ou une question : depot ignore
  const target = type === 'leave' || busy ? null : pad.pickAt(x, y);
  // Lueur seulement sur une cible liable (pas de natives, M1–M3, molette gauche).
  const pc = padCtx();
  if (type !== 'drop') { if (draft) pad.hover(target && !refusal(target, profileId, natives, pc) ? target : null); return; }
  pad.hover(null);
  if (busy) return;
  if (!draft) return toast(app, 'Passer en configuration pour modifier');
  if (!target) return toast(app, 'Déposer le fichier sur une touche ou une molette');
  const out = actionForDrop(paths, target, profileId, natives, draft.sounds().map((s) => s.id), pc);
  if (out.error) return toast(app, out.error);
  if (pc?.editable && !isAction(padEdit.target, profileId, target)) padEdit.target = setAction(padEdit.target, profileId, target); // native -> action
  if (out.sound) draft.addSound(out.sound);
  draft.set(target, 'Press', { action: out.action });
  editPanel.showTrigger(target, 0); // l'action deposee est sur l'appui court
  refreshPad();
  select(target);
  if (out.warning) toast(app, out.warning);
});

initFx();
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
  // Un demarrage du moteur a pu migrer l'ancienne config : on relit (relance si elle etait illisible).
  onUp: async () => {
    if (draft || saving) return;
    if (!cfgError) return reload();
    try { await invoke('get_config'); location.reload(); } catch { /* toujours illisible */ }
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
