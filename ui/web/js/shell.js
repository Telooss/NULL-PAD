// Coque de la fenetre : barre des profils et pastilles d'etat (lien, moteur, enregistrement).
import { linkText, esc } from './view.js';

const N = window.NullCtl;
const LINK_ICON = { usb: 'usb', ble: 'bluetooth', none: 'unplug' };

// slots : profileSlots(config) ; renvoie les mises a jour de la barre.
export function mountShell(slots, activeId) {
  document.getElementById('shell').outerHTML = N.shell({ profiles: slots.map((s) => s && { name: esc(s.name), tag: esc(s.tag) }), active: slots.findIndex((s) => s?.id === activeId) });
  const bar = document.querySelector('.nc-status');
  bar.innerHTML =
    '<span class="nc-pill" data-link></span><span class="nc-pill" data-engine></span>' +
    '<button class="nc-btn is-primary" data-start style="display:none"><i data-lucide="play"></i>Démarrer le moteur</button>' +
    '<span class="nc-pill" data-saved><i data-lucide="check"></i>Enregistré</span>' +
    '<button class="nc-btn" data-configure><i data-lucide="pencil"></i>Configurer</button>' +
    '<button class="nc-btn is-primary" data-save style="display:none"><i data-lucide="save"></i>Enregistrer</button>' +
    '<button class="nc-btn" data-cancel style="display:none"><i data-lucide="x"></i>Annuler</button>' +
    '<button class="nc-btn"><i data-lucide="settings"></i>Réglages</button>';
  const q = (s) => bar.querySelector(s);
  let engineUp = false;
  const pill = (el, icon, text) => { el.innerHTML = `<i data-lucide="${icon}"></i>${text}`; N.icons(); };

  return {
    setLink: (link) => pill(q('[data-link]'), LINK_ICON[link] ?? 'unplug', linkText(link)),
    setEngine(up) {
      engineUp = up;
      pill(q('[data-engine]'), up ? 'cpu' : 'power-off', up ? 'Moteur en marche' : 'Moteur arrêté');
      q('[data-start]').style.display = up ? 'none' : '';
    },
    setSaved(ok, error) {
      const el = q('[data-saved]');
      el.title = error ?? '';
      pill(el, ok ? 'check' : 'alert-triangle', ok ? 'Enregistré' : 'Erreur d’enregistrement');
    },
    setActive: (id) => document.querySelectorAll('.nc-slot').forEach((s, i) => s.classList.toggle('is-active', slots[i]?.id === id)),
    setConfigurable: (ok) => { q('[data-configure]').disabled = !ok; q('[data-configure]').style.opacity = ok ? '' : '0.45'; },
    onSlot: (cb) => document.querySelectorAll('.nc-slot').forEach((s, i) => s.addEventListener('click', () => slots[i] && cb(slots[i].id))),
    onStart: (cb) => q('[data-start]').addEventListener('click', cb),
    onConfigure: (cb) => q('[data-configure]').addEventListener('click', cb),
    engineUp: () => engineUp,
    // Mode configuration : « Configurer » cede la place a « Enregistrer » et « Annuler ».
    setConfigMode(on) {
      q('[data-configure]').style.display = on ? 'none' : '';
      q('[data-save]').style.display = on ? '' : 'none';
      q('[data-cancel]').style.display = on ? '' : 'none';
    },
    onSave: (cb) => q('[data-save]').addEventListener('click', cb),
    onCancel: (cb) => q('[data-cancel]').addEventListener('click', cb),
  };
}
