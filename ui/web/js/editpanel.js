// Panneau d'edition (mode configuration) : onglets de declencheurs, action en cours, catalogue, editeur par type, legende.
// Toutes les modifications vont dans le brouillon ; rien n'est ecrit avant « Enregistrer ».
import { esc, describeAction, panelModel } from './view.js';
import { ACTIONS, TRIGGERS } from './catalog.js';
import { refusal } from './drop.js';
import { defaultAction } from './editors/model.js';
import { renderFields, h } from './editors/render.js';
import { minimap } from './viewpanel.js';
import { isAction, INPUTS } from './pad-model.js';
import { nativeCurrent, nativePicker } from './editors/native.js';

const N = window.NullCtl;
const TAB_LABELS = { Press: 'Appui court', Hold: 'Appui long', DoubleTap: 'Double' };
const ENC_PARTS = [['CW', 'Sens horaire'], ['CCW', 'Sens anti-horaire'], ['PRESS', 'Clic']];
const note = (iconName, text) => h('div', { class: 'nc-note' }, h('i', { 'data-lucide': iconName }), h('span', {}, text));

// ctx : { pickFile(opts), onChange() } — onChange : le brouillon a change (legendes a redessiner).
export function createEditPanel(el, { pickFile, onChange }) {
  let s = null; // { input, draft, profileId, natives, pad }
  let error = null;
  let picking = null; // « input|trigger » dont le catalogue est ouvert
  const tabs = {};

  const binding = (input, trig) => s.draft.get().bindings.find((b) => b.input === input && b.trigger === trig);
  const ctx = { sounds: () => s.draft.sounds(), pickFile };

  function slotsOf(model) {
    const enc = /^ENC_([LR])_/.exec(s.input)?.[1];
    if (enc) return ENC_PARTS.map(([p, title]) => ({ input: `ENC_${enc}_${p}`, title, triggers: p === 'PRESS' ? TRIGGERS : ['Press'] }));
    return model.kind === 'key' || model.kind === 'native' || model.kind === 'profile' ? [{ input: s.input, triggers: TRIGGERS }] : [];
  }

  // Une liaison (entree + declencheur) : catalogue, ou action en cours + editeur + legende.
  function slot(sl, single) {
    const sec = h('div', { class: 'nc-section', style: 'gap:12px' });
    if (sl.title) sec.append(h('span', { class: 'nc-label' }, sl.title));
    const ref = refusal(sl.input, s.profileId, s.natives, s.pad);
    if (ref) { sec.append(note('lock', ref)); return sec; }
    // Pad modifiable : position native (famille « Touche native » seulement), ou touche native choisie dans le catalogue
    // (pas de declencheur : la touche entiere change).
    const nk = `${sl.input}|native`;
    const native = s.pad?.editable && !isAction(s.pad.model, s.profileId, sl.input);
    if (native || picking === nk) {
      const cur = native ? s.pad.model.layers[s.profileId - 1].bindings[INPUTS.indexOf(sl.input)] : null;
      if (picking === nk) sec.append(...nativePicker({ current: cur, catalog: s.pad.catalog, onPick: (nb) => { picking = null; s.pad.toNative(sl.input, nb); }, onCancel: () => { picking = null; render(); } }));
      else sec.append(...nativeCurrent(cur, { onChange: () => { picking = nk; render(); }, onAction: () => s.pad.toAction(sl.input) }));
      return sec;
    }
    const ti = Math.min(tabs[sl.input] ?? 0, sl.triggers.length - 1);
    const trig = sl.triggers[ti];
    if (sl.triggers.length > 1) {
      sec.append(h('div', { class: 'nc-tabs', style: 'margin:0', role: 'tablist' }, sl.triggers.map((t, i) =>
        h('button', { class: i === ti ? 'is-on' : '', role: 'tab', 'aria-selected': String(i === ti), onclick: () => { tabs[sl.input] = i; picking = null; render(); } }, TAB_LABELS[t]))));
    }
    const key = `${sl.input}|${trig}`;
    const b = binding(sl.input, trig);
    if (!b || picking === key) {
      sec.append(h('span', { class: 'nc-label' }, b ? "Changer d'action" : 'Choisir une action'),
        h('div', { class: 'nc-actions', style: 'padding:0' }, ACTIONS.map((a) => h('button', { class: 'nc-action',
          onclick: () => { picking = null; set(sl.input, trig, defaultAction(a.type, ctx.sounds()), {}, true); } },
        h('span', { class: 'ic' }, h('i', { 'data-lucide': a.icon })), h('span', { class: 'tx' }, h('b', {}, a.label), h('span', {}, a.desc)))),
        // Pad modifiable : la touche peut aussi envoyer une touche native (ses actions nullctl seront retirees).
        s.pad?.editable ? h('button', { class: 'nc-action', onclick: () => { picking = `${sl.input}|native`; render(); } },
          h('span', { class: 'ic' }, h('i', { 'data-lucide': 'keyboard' })), h('span', { class: 'tx' }, h('b', {}, 'Touche native'), h('span', {}, 'Envoyée par le pad, sans nullctl'))) : null));
      if (b) sec.append(h('div', { class: 'nc-row' }, h('button', { class: 'nc-btn', onclick: () => { picking = null; render(); } }, 'Garder l’action en cours')));
      return sec;
    }
    const d = describeAction(b.action);
    const desc = h('span', {}, d.desc);
    sec.append(h('div', { class: 'nc-current' }, h('span', { class: 'ic' }, h('i', { 'data-lucide': d.icon })),
      h('span', { class: 'tx' }, h('b', {}, d.title), desc),
      h('button', { class: 'nc-btn', onclick: () => { picking = key; render(); } }, 'Changer')));
    const fields = h('div', { class: 'nc-steps', style: 'gap:16px' });
    renderFields(fields, b.action, ctx, (a, extra, rr) => { set(sl.input, trig, a, extra, rr); if (!rr) desc.textContent = describeAction(a).desc; });
    sec.append(fields);
    sec.append(h('div', { class: 'nc-field' }, h('span', { class: 'nc-label' }, 'Légende personnalisée'),
      h('input', { class: 'nc-input', value: b.label ?? '', placeholder: 'Automatique', maxLength: 24,
        oninput: (e) => {
          const { label, ...rest } = binding(sl.input, trig);
          s.draft.set(sl.input, trig, e.target.value ? { ...rest, label: e.target.value } : rest);
          onChange();
        } })));
    const remove = h('button', { class: 'nc-btn', onclick: () => { s.draft.set(sl.input, trig, null); onChange(); render(); } }, h('i', { 'data-lucide': 'trash-2' }), "Retirer l'action");
    if (single) sec.footBtn = remove; else sec.append(h('div', { class: 'nc-row' }, remove));
    return sec;
  }

  function set(input, trig, action, extra = {}, rr = false) {
    const b = binding(input, trig);
    s.draft.set(input, trig, { ...b, action });
    if (extra.newSound) s.draft.addSound(extra.newSound);
    if (extra.soundPatch) s.draft.updateSound(extra.soundPatch.id, extra.soundPatch.patch);
    onChange();
    if (rr) render();
  }

  function render() {
    if (!s?.input) {
      el.innerHTML = '<div class="nc-panel-head"><div><div class="nc-label">Touche</div><div class="nc-keyname">—</div></div></div>' +
        '<div class="nc-section"><span class="nc-label">Choisis une touche, ou dépose un fichier dessus</span></div>';
      if (error) el.append(h('div', { class: 'nc-section' }, note('alert-triangle', error)));
    } else {
      const m = panelModel(s.draft.merged(), s.profileId, s.input, s.natives, s.pad?.model);
      el.innerHTML = `<div class="nc-panel-head"><div><div class="nc-label">Touche</div><div class="nc-keyname">${esc(m.name)}</div></div>` +
        `<div class="ui-small" style="color:var(--ink-muted)">${esc(m.place)}</div>${minimap(s.input)}</div>`;
      if (error) el.append(h('div', { class: 'nc-section' }, note('alert-triangle', error)));
      const slots = slotsOf(m);
      const secs = slots.map((sl) => slot(sl, slots.length === 1));
      el.append(h('div', { class: 'nc-panel-body' }, secs));
      const foot = secs.find((x) => x.footBtn)?.footBtn;
      if (foot) el.append(h('div', { class: 'nc-panel-foot' }, foot));
    }
    N.icons();
  }

  return {
    // st : { input, draft, profileId, natives, pad } ; input peut etre null.
    // pad : { model, editable, catalog, toNative(input, binding), toAction(input) } ou null.
    show(st) {
      if (st?.input !== s?.input) picking = null;
      s = st;
      render();
    },
    // Erreur de validation ou du moteur, en tete du panneau (null pour l'effacer).
    setError(text) { error = text; render(); },
    // Onglet de declencheur a montrer pour une entree au prochain rendu.
    showTrigger(input, i) { tabs[input] = i; picking = null; },
  };
}
