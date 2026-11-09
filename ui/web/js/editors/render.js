// Rendu DOM des editeurs d'action (champs nc-field, nc-seg, nc-slider, nc-check du design system).
// Aucun texte utilisateur ne passe par innerHTML : tout est pose par textContent / value.
import { fieldsOf, applyField, defaultAction, NESTED_TYPES, TYPE_LABELS, addStep, removeStep, moveStep, setStep, shortcutOf } from './model.js';
import { AUDIO, soundFromPath } from '../drop.js';

export function h(tag, props = {}, ...kids) {
  const e = document.createElement(tag);
  for (const [k, v] of Object.entries(props)) {
    if (k === 'class') e.className = v;
    else if (k.startsWith('on')) e.addEventListener(k.slice(2), v);
    else if (k in e && k !== 'list') e[k] = v;
    else e.setAttribute(k, v);
  }
  for (const c of kids.flat()) if (c != null && c !== false) e.append(c);
  return e;
}
export const icon = (name) => h('i', { 'data-lucide': name });
const iconBtn = (name, title, onclick) => h('button', { class: 'nc-btn nc-btn-sm', title, 'aria-label': title, onclick }, icon(name));
export const fieldBox = (label, ...kids) => h('div', { class: 'nc-field' }, h('span', { class: 'nc-label' }, label), ...kids);
const base = (p) => p.split(/[\\/]/).pop();

// ctx : { sounds() -> SoundDef[], pickFile({filters}) -> Promise<chemin|null> }
// onChange(action, extra, rerender) ; extra : { soundPatch?, newSound? }.
export function renderFields(host, action, ctx, onChange, { nested = false } = {}) {
  let cur = action;
  const emit = (a, extra = {}, rr = false) => { cur = a; onChange(a, extra, rr); };
  const apply = (key, value, rr) => {
    const r = applyField(cur, key, value, ctx.sounds());
    emit(r.action, r.soundPatch ? { soundPatch: r.soundPatch } : {}, rr);
    return r;
  };

  if (nested) {
    const sel = h('select', { class: 'nc-input', 'aria-label': "Type d'action", onchange: () => emit(defaultAction(sel.value, ctx.sounds()), {}, true) },
      NESTED_TYPES.map((t) => h('option', { value: t, selected: t === cur.type }, TYPE_LABELS[t])));
    host.append(sel);
  }

  for (const f of fieldsOf(cur, ctx.sounds())) {
    if (nested && f.key === 'type') continue;
    switch (f.kind) {
      case 'seg':
        host.append(fieldBox(f.label, h('div', { class: 'nc-seg', role: 'group' }, f.options.map((o) =>
          h('button', { class: o.value === f.value ? 'is-on' : '', 'aria-pressed': String(o.value === f.value), onclick: () => apply(f.key, o.value, true) }, o.label)))));
        break;
      case 'text':
        host.append(fieldBox(f.label, h('input', { class: 'nc-input', value: f.value, oninput: (e) => apply(f.key, e.target.value) })));
        break;
      case 'textarea': case 'lines':
        host.append(fieldBox(f.label, h('textarea', { class: 'nc-input', rows: 3, value: f.value, oninput: (e) => apply(f.key, e.target.value) })));
        break;
      case 'number': {
        const inp = h('input', { class: 'nc-input', type: 'number', min: f.min, max: f.max, value: f.value,
          onchange: () => { inp.value = apply(f.key, inp.value).action[f.key]; } });
        host.append(fieldBox(`${f.label} (${f.min} à ${f.max})`, inp));
        break;
      }
      case 'check': {
        const toggle = () => apply(f.key, !f.value, true);
        host.append(h('div', { class: `nc-check${f.value ? ' is-on' : ''}`, role: 'checkbox', tabIndex: 0, 'aria-checked': String(f.value),
          onclick: toggle, onkeydown: (e) => { if (e.key === ' ' || e.key === 'Enter') { e.preventDefault(); toggle(); } } },
        h('span', { class: 'box' }, f.value ? icon('check') : null), h('span', { class: 'tx' }, f.label)));
        break;
      }
      case 'file': {
        const inp = h('input', { class: 'nc-input', value: f.value, oninput: () => apply(f.key, inp.value) });
        const filters = f.key === 'program' ? [{ name: 'Programmes', extensions: ['exe', 'bat', 'cmd'] }] : [];
        const browse = async () => {
          const p = await ctx.pickFile({ filters });
          if (p) { inp.value = p; apply(f.key, p); }
        };
        host.append(fieldBox(f.label, h('div', { class: 'nc-row' }, inp, h('button', { class: 'nc-btn', onclick: browse }, icon('folder-open'), 'Parcourir'))));
        break;
      }
      case 'keys': {
        const inp = h('input', { class: 'nc-input nc-mono', readOnly: true, value: f.value, placeholder: 'Appuyer sur les touches…',
          onkeydown: (e) => {
            if (e.key === 'Tab' && !e.ctrlKey && !e.altKey) return; // laisse Tab naviguer
            e.preventDefault();
            const s = shortcutOf(e);
            if (s) { inp.value = s; apply(f.key, s); }
          } });
        host.append(fieldBox(f.label, inp));
        break;
      }
      case 'slider': {
        const out = h('output', {}, `${f.value} %`);
        const fill = h('div', { class: 'fill', style: `width:${f.value}%` });
        const knob = h('div', { class: 'knob', style: `left:${f.value}%` });
        const rng = h('input', { type: 'range', min: f.min, max: f.max, value: f.value, 'aria-label': f.label,
          oninput: () => {
            const v = apply(f.key, rng.value).soundPatch?.patch[f.key] ?? +rng.value;
            out.textContent = `${v} %`; fill.style.width = `${v}%`; knob.style.left = `${v}%`;
          } });
        host.append(fieldBox(f.label, h('div', { class: 'nc-slider' }, h('div', { class: 'track' }, fill, knob, rng), out)));
        break;
      }
      case 'sound': {
        const sel = h('select', { class: 'nc-input', 'aria-label': f.label,
          onchange: async () => {
            if (sel.value !== '__new') return apply(f.key, sel.value, true);
            const p = await ctx.pickFile({ filters: [{ name: 'Audio', extensions: AUDIO }] });
            if (!p) { sel.value = f.value; return; }
            const def = soundFromPath(p, ctx.sounds().map((s) => s.id));
            emit({ ...cur, id: def.id }, { newSound: def }, true);
          } },
        f.value && !f.options.some((o) => o.value === f.value) ? h('option', { value: f.value, selected: true }, `${f.value} (absent)`) : null,
        !f.value ? h('option', { value: '', selected: true }, '— choisir —') : null,
        f.options.map((o) => h('option', { value: o.value, selected: o.value === f.value }, o.label)),
        h('option', { value: '__new' }, 'Nouveau fichier…'));
        const path = f.options.find((o) => o.value === f.value)?.path;
        host.append(fieldBox(f.label, sel, path ? h('div', { class: 'nc-file' }, icon('file-audio'),
          h('span', { class: 'tx' }, h('b', {}, base(path)), h('span', {}, path))) : null));
        break;
      }
      case 'sub': {
        const box = h('div', { class: 'nc-step' });
        renderFields(box, f.value ?? defaultAction('MicMute'), ctx, (a, extra, rr) => emit(applyField(cur, f.key, a).action, extra, rr), { nested: true });
        host.append(fieldBox(f.label, box));
        break;
      }
      case 'steps':
        host.append(fieldBox(f.label, renderSteps(() => cur.steps ?? [], ctx, (steps, extra, rr) => emit({ ...cur, steps }, extra, rr))));
        break;
    }
  }
}

// Etapes Faire / Attendre : ajout, suppression, deplacement.
function renderSteps(steps, ctx, change) {
  const list = h('div', { class: 'nc-steps' });
  steps().forEach((s, i) => {
    const body = h('div', { class: 'nc-steps' });
    if (s.type === 'Wait') {
      const inp = h('input', { class: 'nc-input', type: 'number', min: 0, max: 600000, value: s.ms, 'aria-label': 'Attente en ms',
        onchange: () => {
          const ms = Math.max(0, Math.min(600000, Math.round(+inp.value) || 0));
          inp.value = ms;
          change(setStep(steps(), i, { type: 'Wait', ms }), {}, false);
        } });
      body.append(h('div', { class: 'nc-row' }, inp, h('span', { class: 'nc-label' }, 'ms')));
    } else {
      renderFields(body, s.action, ctx, (a, extra, rr) => change(setStep(steps(), i, { type: 'Do', action: a }), extra, rr), { nested: true });
    }
    const n = steps().length;
    list.append(h('div', { class: 'nc-step' },
      h('div', { class: 'nc-row' }, h('span', { class: 'nc-label', style: 'flex:1' }, `${i + 1}. ${s.type === 'Wait' ? 'Attendre' : 'Faire'}`),
        i > 0 ? iconBtn('arrow-up', 'Monter', () => change(moveStep(steps(), i, -1), {}, true)) : null,
        i < n - 1 ? iconBtn('arrow-down', 'Descendre', () => change(moveStep(steps(), i, 1), {}, true)) : null,
        iconBtn('x', "Supprimer l'étape", () => change(removeStep(steps(), i), {}, true))),
      body));
  });
  list.append(h('div', { class: 'nc-row' },
    h('button', { class: 'nc-btn', onclick: () => change(addStep(steps(), 'Do'), {}, true) }, icon('plus'), 'Faire'),
    h('button', { class: 'nc-btn', onclick: () => change(addStep(steps(), 'Wait'), {}, true) }, icon('timer'), 'Attendre')));
  return list;
}
