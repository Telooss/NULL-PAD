// Panneau lecture seule d'une touche (classes nc-* du design system, aucun controle d'edition).
import { esc, INPUTS } from './view.js';

const N = window.NullCtl;

const current = (a) => `<div class="nc-current"><span class="ic"><i data-lucide="${a.icon}"></i></span>` +
  `<span class="tx"><b>${esc(a.title)}</b><span>${esc(a.desc)}${a.label ? ` · « ${esc(a.label)} »` : ''}</span></span></div>`;
const none = '<div class="nc-note"><i data-lucide="minus"></i><span>Aucune action</span></div>';
const note = (t) => `<div class="nc-note"><i data-lucide="info"></i><span>${esc(t)}</span></div>`;

export function minimap(input) {
  let m = '';
  for (let i = 0; i < 20; i++) m += `<i class="${INPUTS[i] === input ? 'sel' : ''}"></i>`;
  return `<div class="nc-minimap">${m}<span class="mrow">${['M1', 'M2', 'M3'].map((k) => `<i class="v${k === input ? ' sel' : ''}"></i>`).join('')}</span></div>`;
}

export function createViewPanel(el, { onConfigure }) {
  let model = null;
  let tab = 0;
  let blocked = false;

  const foot = () => `<div class="nc-panel-foot"><button class="nc-btn" data-configure${blocked ? ' disabled style="opacity:.45"' : ''}><i data-lucide="pencil"></i>Configurer</button></div>`;
  const section = (s, withTabs) => {
    const rows = withTabs ? [s.rows[tab]] : s.rows;
    const body = s.native ? note(s.native) : rows.map((r) => `${withTabs ? '' : `<span class="nc-label">${esc(r.label)}</span>`}${r.action ? current(r.action) : none}`).join('');
    return `<div class="nc-section" style="gap:10px">${s.title ? `<span class="nc-label">${esc(s.title)}</span>` : ''}${body}</div>`;
  };

  function render() {
    if (!model) {
      el.innerHTML = '<div class="nc-panel-head"><div><div class="nc-label">Touche</div><div class="nc-keyname">—</div></div></div>' +
        '<div class="nc-section"><span class="nc-label">Choisis une touche</span></div>' + foot();
    } else {
      const keyed = model.kind === 'key';
      el.innerHTML = `<div class="nc-panel-head"><div><div class="nc-label">Touche</div><div class="nc-keyname">${esc(model.name)}</div></div>` +
        `<div class="ui-small" style="color:var(--ink-muted)">${esc(model.place)}</div>${minimap(model.input)}</div>` +
        (keyed ? `<div class="nc-tabs">${model.sections[0].rows.map((r, i) => `<button data-tab="${i}"${i === tab ? ' class="is-on"' : ''}>${esc(r.label)}</button>`).join('')}</div>` : '') +
        (model.note ? `<div class="nc-section">${note(model.note)}</div>` : '') +
        model.sections.map((s) => section(s, keyed)).join('') + foot();
    }
    el.querySelectorAll('[data-tab]').forEach((b) => b.addEventListener('click', () => { tab = +b.dataset.tab; render(); N.wire(el.parentElement); }));
    el.querySelector('[data-configure]').addEventListener('click', () => { if (!blocked) onConfigure(); });
    N.icons();
  }

  render();
  return {
    // m : panelModel(...) + input, ou null.
    show(m) { if (m?.input !== model?.input) tab = 0; model = m; render(); },
    // Config illisible : tout reste en lecture seule et « Configurer » est inactif.
    block(text) {
      blocked = true;
      model = null;
      render();
      el.querySelector('.nc-section').innerHTML = `<div class="nc-note"><i data-lucide="alert-triangle"></i><span>${esc(text)}</span></div>`;
      N.icons();
    },
  };
}
