// Coque de la fenetre : barre des profils et pastilles d'etat (lien, moteur, enregistrement).
import { linkText, esc } from './view.js';
import { slotLabel } from './profiles.js';

const N = window.NullCtl;
const LINK_ICON = { usb: 'usb', ble: 'bluetooth', none: 'unplug' };
// Pastille du pad (pillState de pad-model.js) : [icone, texte].
const PAD_PILL = { editable: ['unlock', 'Pad modifiable'], locked: ['lock', 'Pad verrouillé'], absent: ['unplug', 'Pad non branché'],
  stale: ['alert-triangle', 'Pad pas à jour'], busy: ['loader', 'Pad occupé'] };

// Barre des profils : setSlots(barSlots(config), actif) a chaque changement ; renvoie les mises a jour de la barre.
export function mountShell() {
  document.getElementById('shell').outerHTML = N.shell({ profiles: [], active: -1 });
  const bar = document.querySelector('.nc-status');
  bar.innerHTML =
    '<span class="nc-pill" data-link></span><span class="nc-pill" data-pad></span><span class="nc-pill" data-engine></span>' +
    '<button class="nc-btn is-primary" data-start style="display:none" title="Démarrer le moteur"><i data-lucide="play"></i><span class="lbl">Démarrer le moteur</span></button>' +
    '<span class="nc-pill" data-saved title="Enregistré"><i data-lucide="check"></i><span class="lbl">Enregistré</span></span>' +
    '<button class="nc-btn" data-configure title="Configurer"><i data-lucide="pencil"></i><span class="lbl">Configurer</span></button>' +
    '<button class="nc-btn is-primary" data-save style="display:none"><i data-lucide="save"></i>Enregistrer</button>' +
    '<button class="nc-btn" data-cancel style="display:none"><i data-lucide="x"></i>Annuler</button>' +
    '<button class="nc-btn" data-settings title="Réglages"><i data-lucide="settings"></i><span class="lbl">Réglages</span></button>';
  // Libelles repliables sous 1400 px (app.css) ; l'infobulle garde le texte.
  const np = document.querySelector('.nc-newprofile');
  np.title = np.textContent.trim();
  np.lastChild.replaceWith(Object.assign(document.createElement('span'), { className: 'lbl', textContent: np.title }));
  const q = (s) => bar.querySelector(s);
  const topbar = document.querySelector('.nc-topbar');
  let engineUp = false;
  let active = null;
  const cbs = { slot: () => {}, newProfile: () => {}, menu: () => {} };
  const slotEl = (t) => t.closest('.nc-slot');
  topbar.addEventListener('click', (e) => {
    const s = slotEl(e.target);
    if (s) return s.dataset.id ? cbs.slot(+s.dataset.id) : cbs.newProfile(+s.dataset.empty); // emplacement vide : creer ici
    const m = e.target.closest('.nc-slotmore');
    if (m) cbs.menu(+m.dataset.id, m);
  });
  // Clic droit (ou touche Menu / Maj+F10 sur l'emplacement au focus) : meme menu que « ⋯ ».
  topbar.addEventListener('contextmenu', (e) => {
    const s = slotEl(e.target);
    if (!s?.dataset.id || s.dataset.id === '1') return;
    e.preventDefault();
    cbs.menu(+s.dataset.id, s);
  });
  const pill = (el, icon, text) => { el.innerHTML = `<i data-lucide="${icon}"></i><span class="lbl">${text}</span>`; el.title = text; N.icons(); };

  return {
    setLink: (link) => pill(q('[data-link]'), LINK_ICON[link] ?? 'unplug', linkText(link)),
    setPad: (state) => pill(q('[data-pad]'), ...(PAD_PILL[state] ?? PAD_PILL.absent)),
    setEngine(up) {
      engineUp = up;
      pill(q('[data-engine]'), up ? 'cpu' : 'power-off', up ? 'Moteur en marche' : 'Moteur arrêté');
      q('[data-start]').style.display = up ? 'none' : '';
    },
    setSaved(ok, error) {
      const el = q('[data-saved]');
      pill(el, ok ? 'check' : 'alert-triangle', ok ? 'Enregistré' : 'Erreur d’enregistrement');
      if (error) el.title = error;
    },
    // list : barSlots(config) ; les noms passent par esc (jamais de HTML brut).
    setSlots(list, id) {
      active = id;
      topbar.querySelectorAll('.nc-slot, .nc-slotmore').forEach((n) => n.remove());
      const html = list.map((s, i) => {
        if (!s) return `<button class="nc-slot" data-empty="${i + 1}" title="Créer un profil"><span class="nc-m">M${i + 1}</span><span>Vide</span><i data-lucide="plus" style="margin-left:auto"></i></button>`;
        return `<button class="nc-slot${s.id === active ? ' is-active' : ''}" data-id="${s.id}"><span class="nc-m">${slotLabel(s.id)}</span><span class="nc-sticker">${esc(s.name)}</span><span class="nc-tag">${esc(s.tag)}</span></button>` +
          (s.id === 1 ? '' : `<button class="nc-slotmore" data-id="${s.id}" aria-haspopup="menu" aria-label="Menu du profil ${esc(s.name)}" title="Menu du profil">⋯</button>`);
      }).join('');
      topbar.querySelector('.nc-newprofile').insertAdjacentHTML('beforebegin', html);
      N.icons();
    },
    setActive(id) {
      active = id;
      topbar.querySelectorAll('.nc-slot[data-id]').forEach((s) => s.classList.toggle('is-active', +s.dataset.id === id));
    },
    setCanCreate(ok) {
      np.disabled = !ok;
      np.title = ok ? 'Nouveau profil' : '8 profils au maximum : supprimez-en un pour en créer un nouveau';
    },
    onNew: (cb) => { cbs.newProfile = cb; np.addEventListener('click', () => cb()); },
    onMenu: (cb) => { cbs.menu = cb; },
    setConfigurable: (ok) => { q('[data-configure]').disabled = !ok; q('[data-configure]').style.opacity = ok ? '' : '0.45'; },
    onSlot: (cb) => { cbs.slot = cb; },
    onStart: (cb) => q('[data-start]').addEventListener('click', cb),
    onConfigure: (cb) => q('[data-configure]').addEventListener('click', cb),
    engineUp: () => engineUp,
    // Mode configuration : « Configurer » cede la place a « Enregistrer » et « Annuler ».
    setConfigMode(on) {
      q('[data-configure]').style.display = on ? 'none' : '';
      q('[data-save]').style.display = on ? '' : 'none';
      q('[data-cancel]').style.display = on ? '' : 'none';
    },
    onSettings: (cb) => q('[data-settings]').addEventListener('click', cb),
    onSave: (cb) => q('[data-save]').addEventListener('click', cb),
    onCancel: (cb) => q('[data-cancel]').addEventListener('click', cb),
  };
}
