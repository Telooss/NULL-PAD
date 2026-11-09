// Famille « Touche native » (pad modifiable) : touche en cours et selecteur du catalogue HID.
// Tout texte venant du pad ou du catalogue passe par textContent (h()).
import { h, icon, fieldBox } from './render.js';
import { MODS, groupCatalog, nativeOf, usageOfCode, padLegend } from '../pad-model.js';

const wrapRow = (kids) => h('div', { class: 'nc-row', style: 'flex-wrap:wrap;gap:4px' }, kids);

export function describeNative(b) {
  if (b.kind === 'profile_macro') return `Aller au profil ${b.n}`;
  if (b.kind === 'native') return b.label;
  return `Comportement ZMK : ${padLegend(b)}`;
}

// Touche native en cours : « Changer » ouvre le selecteur, « Rendre a nullctl » la repasse en action.
export function nativeCurrent(b, { onChange, onAction }) {
  return [
    h('div', { class: 'nc-current' }, h('span', { class: 'ic' }, icon('keyboard')),
      h('span', { class: 'tx' }, h('b', {}, 'Touche native'), h('span', {}, describeNative(b))),
      h('button', { class: 'nc-btn', onclick: onChange }, 'Changer')),
    h('div', { class: 'nc-row' }, h('button', { class: 'nc-btn', onclick: onAction }, icon('undo-2'), 'Rendre à nullctl (action)')),
  ];
}

// Selecteur : recherche, capture d'une touche, modificateurs, « Profil N », puis le catalogue par groupe.
// ctx : { current, catalog, onPick(PadBinding), onCancel? }
export function nativePicker({ current, catalog, onPick, onCancel }) {
  const mods = new Set(current?.kind === 'native' ? MODS.filter(([, , bit]) => (current.usage >>> 24) & bit).map(([k]) => k) : []);
  const list = h('div', { class: 'nc-steps', style: 'gap:10px' });
  const pick = (entry) => onPick(nativeOf(entry, [...mods]));
  const keyBtn = (k) => {
    const on = current?.kind === 'native' && current.usage === nativeOf(k, [...mods]).usage;
    return h('button', { class: `nc-btn nc-btn-sm${on ? ' is-primary' : ''}`, 'aria-pressed': String(on), onclick: () => pick(k) }, k.label);
  };
  const fill = (q) => {
    const groups = groupCatalog(catalog, q);
    list.replaceChildren(...groups.map(([g, ks]) => fieldBox(g, wrapRow(ks.map(keyBtn)))));
    if (!groups.length) list.append(h('span', { class: 'nc-label' }, 'Aucune touche'));
  };
  const search = h('input', { class: 'nc-input', type: 'search', placeholder: 'Chercher une touche', 'aria-label': 'Chercher une touche', oninput: () => fill(search.value) });

  // Capture : une seule touche (les modificateurs tenus comptent), Tab compris ; Echap annule.
  // Ecoute en phase de capture sur le document : passe avant les champs (recherche) et les arrete.
  let armed = false;
  const cap = h('button', { class: 'nc-btn', onclick: () => {
    if (armed) return;
    armed = true;
    cap.lastChild.textContent = 'Appuie sur une touche…';
    const key = (e) => {
      if (!cap.isConnected) { document.removeEventListener('keydown', key, true); return; } // panneau redessine
      const usage = usageOfCode(e.code);
      if (usage === null && /^(Control|Shift|Alt|Meta|OS)/.test(e.code)) return; // modificateur seul : on attend la touche
      e.preventDefault();
      e.stopPropagation();
      document.removeEventListener('keydown', key, true);
      armed = false;
      if (e.key === 'Escape') { cap.lastChild.textContent = 'Capturer'; return; }
      const entry = catalog.find((k) => k.usage === usage);
      if (!entry) { cap.lastChild.textContent = 'Touche inconnue'; return; }
      onPick(nativeOf(entry, [e.ctrlKey && 'ctrl', e.shiftKey && 'shift', e.altKey && 'alt', e.metaKey && 'win'].filter(Boolean)));
    };
    document.addEventListener('keydown', key, true);
  } }, icon('keyboard'), h('span', {}, 'Capturer'));

  const modSeg = h('div', { class: 'nc-seg', role: 'group' }, MODS.map(([k, label]) => {
    const b = h('button', { class: mods.has(k) ? 'is-on' : '', 'aria-pressed': String(mods.has(k)), onclick: () => {
      if (mods.has(k)) mods.delete(k); else mods.add(k);
      b.classList.toggle('is-on', mods.has(k));
      b.setAttribute('aria-pressed', String(mods.has(k)));
      fill(search.value);
    } }, label);
    return b;
  }));
  const profiles = h('div', { class: 'nc-seg', role: 'group' }, Array.from({ length: 8 }, (_, i) => {
    const on = current?.kind === 'profile_macro' && current.n === i + 1;
    return h('button', { class: on ? 'is-on' : '', 'aria-pressed': String(on), onclick: () => onPick({ kind: 'profile_macro', n: i + 1 }) }, `P${i + 1}`);
  }));
  fill('');
  return [
    fieldBox('Touche native', h('div', { class: 'nc-row' }, search, cap)),
    fieldBox('Avec', modSeg),
    fieldBox('Aller au profil', profiles),
    list,
    onCancel ? h('div', { class: 'nc-row' }, h('button', { class: 'nc-btn', onclick: onCancel }, 'Garder la touche en cours')) : null,
  ];
}
