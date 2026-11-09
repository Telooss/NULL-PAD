// Mode configuration : sauvegarde (machine a etats pure), garde des changements non enregistres, boite de confirmation.

import { parseErr, samePad } from './pad-model.js';

// deps : { json, validate(json, pad), save(json, pad), waitConfigEvent(ms) -> { promise -> {ok,error}|null, cancel() }, engineUp() -> bool, discard(), pad? }
// pad (pad modifiable) : { expected, target, write(expected, target) -> PadModel, onForeign() -> bool (abandon confirme et fait) }.
// -> { state: 'saved' | 'saved-offline' | 'invalid' | 'rejected' | 'pad', error?, code? }. `discard` (sortie du mode,
// brouillon jete) n'est appele que pour saved / saved-offline : sinon le brouillon reste. `write: true` : echec d'ecriture.
// Le pad d'abord (si la cible differe de la lecture), la config ensuite : un pad qui refuse ne fait perdre aucune action.
// Apres un pad ecrit : `padUpdated: true` et `padModel` (relu par Rust), quel que soit l'etat de la config.
export async function saveFlow({ json, validate, save, waitConfigEvent, engineUp, discard = () => {}, pad = null }) {
  let model = pad?.target ?? null;
  try {
    await validate(json, model);
  } catch (e) {
    return { state: 'invalid', error: String(e) };
  }
  let extra = {};
  if (pad && !samePad(pad.expected, pad.target)) {
    for (let retried = false; ; retried = true) {
      try {
        model = await pad.write(pad.expected, pad.target);
        break;
      } catch (e) {
        const err = parseErr(e);
        // Changements laisses par ZMK Studio : abandonnes seulement apres confirmation, puis un seul nouvel essai.
        let again = false;
        if (err.code === 'foreign-unsaved' && !retried) {
          try { again = await pad.onForeign(); } catch (e2) { const e3 = parseErr(e2); return { state: 'pad', code: e3.code, error: e3.message }; }
        }
        if (!again) return { state: 'pad', code: err.code, error: err.message };
      }
    }
    extra = { padUpdated: true, padModel: model };
  }
  // Ecoute armee avant l'ecriture : l'evenement `config` peut arriver tout de suite.
  const wait = engineUp() ? waitConfigEvent(3000) : null;
  try {
    await save(json, model);
  } catch (e) {
    wait?.cancel();
    return { state: 'invalid', error: String(e), write: true, ...extra }; // ecriture refusee : rien n'a change
  }
  const ev = wait && (await wait.promise);
  if (ev && !ev.ok) return { state: 'rejected', error: ev.error ?? 'refusé par le moteur', ...extra };
  const out = { state: ev ? 'saved' : 'saved-offline', ...extra };
  discard();
  return out;
}

// Quitter le brouillon (annuler, changer de profil, fermer) : demander seulement s'il est modifie.
export const guard = (dirty, action) => (dirty && action !== 'save' ? 'ask' : 'proceed');

export const modalOpen = () => !!document.querySelector('.nc-modal');

// Boite modale stylee nc-panel. buttons : [[valeur, libelle, principal?]] ; Echap = dernier bouton.
// Une seule a la fois : si une boite est deja ouverte, la reponse est le dernier bouton (« Rester »).
// La promesse rendue a close(valeur) pour fermer la boite depuis le code (attente du pad finie).
export function confirmBox(host, { title, text, buttons }) {
  const N = window.NullCtl;
  if (modalOpen()) return Object.assign(Promise.resolve(buttons.at(-1)[0]), { close() {} });
  let close;
  const p = new Promise((resolve) => {
    const wrap = document.createElement('div');
    wrap.className = 'nc-modal';
    wrap.innerHTML = '<div class="nc-panel" role="dialog" aria-modal="true"><div class="nc-panel-head"><div><div class="nc-label"></div></div></div>' +
      '<div class="nc-section" style="padding-bottom:14px"><div class="nc-note"><i data-lucide="alert-triangle"></i><span></span></div></div><div class="nc-panel-foot"></div></div>';
    wrap.querySelector('.nc-label').textContent = title;
    wrap.querySelector('.nc-note span').textContent = text;
    const foot = wrap.querySelector('.nc-panel-foot');
    const done = (v) => { wrap.remove(); document.removeEventListener('keydown', key, true); resolve(v); };
    close = done;
    const key = (e) => {
      if (e.key === 'Escape') { e.stopPropagation(); done(buttons.at(-1)[0]); }
      if (e.key !== 'Tab') return;
      // Focus garde dans la boite.
      const bs = [...foot.querySelectorAll('button')];
      const i = bs.indexOf(document.activeElement);
      e.preventDefault();
      bs[(i + (e.shiftKey ? -1 : 1) + bs.length) % bs.length].focus();
    };
    for (const [value, label, primary] of buttons) {
      const b = document.createElement('button');
      b.className = `nc-btn${primary ? ' is-primary' : ''}`;
      b.textContent = label;
      b.onclick = () => done(value);
      foot.appendChild(b);
    }
    document.addEventListener('keydown', key, true);
    host.appendChild(wrap);
    N.icons();
    foot.querySelector('.is-primary, .nc-btn')?.focus();
  });
  p.close = (v) => close(v);
  return p;
}
