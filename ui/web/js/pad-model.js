// Keymap du pad (PadModel lu par Rust) cote interface. Pur : aucun acces DOM.
// PadBinding : {kind:'action',profile,input} | {kind:'native',usage,label} | {kind:'profile_macro',n} | {kind:'other',behavior,p1,p2}
import { canBind as firmwareCanBind } from './draft.js';

// Index = position du pad (0-28) ; profil p = couche p-1.
export const INPUTS = [...Array.from({ length: 20 }, (_, i) => `K${String(i).padStart(2, '0')}`), 'M1', 'M2', 'M3',
  'ENC_L_CW', 'ENC_L_CCW', 'ENC_L_PRESS', 'ENC_R_CW', 'ENC_R_CCW', 'ENC_R_PRESS'];

const pos = (input) => INPUTS.indexOf(input);
const bindingAt = (model, p, input) => model?.layers[p - 1]?.bindings[pos(input)];

// Meme regle que store::is_native cote Rust : seule Action{p, i} a sa place est une action nullctl.
export function isAction(model, p, input) {
  const b = bindingAt(model, p, input);
  return b?.kind === 'action' && b.profile === p && b.input === pos(input);
}

// Legende d'une liaison du pad ; null pour une action a sa place (legende nullctl).
export function padLegend(b) {
  switch (b?.kind) {
    case 'native': return b.label.toUpperCase();
    case 'profile_macro': return `P${b.n}`;
    case 'other': return b.behavior === 'np_00' ? '00' : b.behavior;
    case 'action': return null;
    default: return '?';
  }
}

// Entrees natives d'un profil d'apres le pad : { input: legende }.
export function padNatives(model, p) {
  const out = {};
  for (const input of INPUTS) {
    if (isAction(model, p, input)) continue;
    const b = bindingAt(model, p, input);
    if (b) out[input] = padLegend(b) ?? 'ACT';
  }
  return out;
}

// pad : { model, editable } ou null. Pad modifiable : tout s'edite (action ou touche native).
// Keymap connue mais pas modifiable : seules les actions a leur place. Sans keymap : table du firmware.
export function canBind(profileId, input, natives = {}, pad = null) {
  if (pad?.editable) return true;
  if (pad?.model) return isAction(pad.model, profileId, input);
  return firmwareCanBind(profileId, input, natives);
}

const withBinding = (model, p, input, b) => {
  const out = structuredClone(model);
  out.layers[p - 1].bindings[pos(input)] = structuredClone(b);
  return out;
};

// Passer en native : ses liaisons nullctl dans ce profil ne pourraient plus se declencher, on les retire.
export function setNative(target, draft, p, input, binding) {
  for (const b of draft.get().bindings.filter((x) => x.input === input)) draft.set(input, b.trigger, null);
  return withBinding(target, p, input, binding);
}

export const setAction = (target, p, input) => withBinding(target, p, input, { kind: 'action', profile: p, input: pos(input) });

// Le nom de couche recoit le tag (affiche par l'ecran du pad).
export function renameLayer(target, p, tag, maxLen) {
  const out = structuredClone(target);
  out.layers[p - 1].name = tag.slice(0, maxLen);
  return out;
}

export function withLayer(model, p, bindings, name) {
  const out = structuredClone(model);
  Object.assign(out.layers[p - 1], { bindings: structuredClone(bindings), name });
  return out;
}

// Copie d'une couche : les actions du profil source deviennent celles du profil cible.
export const copyLayer = (bindings, src, dst) =>
  bindings.map((b) => (b.kind === 'action' && b.profile === src ? { ...b, profile: dst } : structuredClone(b)));

// Erreur Rust « <code>: <message> ».
export function parseErr(e) {
  const s = String(e?.message ?? e);
  const m = /^([a-z-]+): ([\s\S]*)$/.exec(s);
  return m ? { code: m[1], message: m[2] } : { code: null, message: s };
}

export const samePad = (a, b) => JSON.stringify(a) === JSON.stringify(b);

// Pastille : stale (operation faite sans le pad) passe avant le reste.
export function pillState(status, stale) {
  if (stale) return 'stale';
  if (!status) return 'absent';
  if (status.busy) return 'busy';
  if (!status.present) return 'absent';
  return status.locked ? 'locked' : 'editable';
}

// Modificateurs implicites ZMK (octet haut de l'usage) : LCTL 0x01, LSFT 0x02, LALT 0x04, LGUI 0x08.
export const MODS = [['ctrl', 'Ctrl', 0x01], ['shift', 'Maj', 0x02], ['alt', 'Alt', 0x04], ['win', 'Win', 0x08]];
export function nativeOf(entry, mods = []) {
  const on = MODS.filter(([k]) => mods.includes(k));
  const bits = on.reduce((m, [, , bit]) => m | bit, 0);
  return { kind: 'native', usage: (entry.usage | (bits << 24)) >>> 0, label: [...on.map(([, l]) => l), entry.label].join('+') };
}

// KeyboardEvent.code -> usage HID (page clavier 0x07) ; null pour un modificateur seul ou une touche inconnue.
const CODES = { Enter: 0x28, Escape: 0x29, Backspace: 0x2a, Tab: 0x2b, Space: 0x2c, Minus: 0x2d, Equal: 0x2e, BracketLeft: 0x2f,
  BracketRight: 0x30, Backslash: 0x31, Semicolon: 0x33, Quote: 0x34, Backquote: 0x35, Comma: 0x36, Period: 0x37, Slash: 0x38,
  CapsLock: 0x39, PrintScreen: 0x46, ScrollLock: 0x47, Pause: 0x48, Insert: 0x49, Home: 0x4a, PageUp: 0x4b, Delete: 0x4c, End: 0x4d,
  PageDown: 0x4e, ArrowRight: 0x4f, ArrowLeft: 0x50, ArrowDown: 0x51, ArrowUp: 0x52, NumLock: 0x53, NumpadDivide: 0x54,
  NumpadMultiply: 0x55, NumpadSubtract: 0x56, NumpadAdd: 0x57, NumpadEnter: 0x58, Numpad0: 0x62, NumpadDecimal: 0x63,
  ContextMenu: 0x65, NumpadEqual: 0x67, Digit0: 0x27 };
export function usageOfCode(code) {
  let id = CODES[code];
  let m;
  if (id === undefined && (m = /^Key([A-Z])$/.exec(code))) id = 4 + m[1].charCodeAt(0) - 65;
  else if (id === undefined && (m = /^Digit([1-9])$/.exec(code))) id = 0x1e + +m[1] - 1;
  else if (id === undefined && (m = /^Numpad([1-9])$/.exec(code))) id = 0x59 + +m[1] - 1;
  else if (id === undefined && (m = /^F(\d+)$/.exec(code)) && +m[1] >= 1 && +m[1] <= 24) id = +m[1] <= 12 ? 0x3a + +m[1] - 1 : 0x68 + +m[1] - 13;
  return id === undefined ? null : (0x07 << 16) | id;
}

const fold = (s) => String(s).normalize('NFD').replace(/[̀-ͯ]/g, '').toLowerCase();

// Catalogue filtre (libelle ou groupe), regroupe dans l'ordre : [[groupe, entrees]].
export function groupCatalog(catalog, query = '') {
  const q = fold(query.trim());
  const out = new Map();
  for (const k of catalog) {
    if (q && !fold(k.label).includes(q) && !fold(k.group).includes(q)) continue;
    if (!out.has(k.group)) out.set(k.group, []);
    out.get(k.group).push(k);
  }
  return [...out];
}
