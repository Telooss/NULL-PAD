// Mode configuration : sauvegarde (machine a etats pure), garde des changements non enregistres, boite de confirmation.

// deps : { json, validate(json), save(json), waitConfigEvent(ms) -> { promise -> {ok,error}|null, cancel() }, engineUp() -> bool, discard() }
// -> { state: 'saved' | 'saved-offline' | 'invalid' | 'rejected', error? }. `discard` (sortie du mode,
// brouillon jete) n'est appele que pour saved / saved-offline : sinon le brouillon reste. `write: true` : echec d'ecriture.
export async function saveFlow({ json, validate, save, waitConfigEvent, engineUp, discard = () => {} }) {
  try {
    await validate(json);
  } catch (e) {
    return { state: 'invalid', error: String(e) };
  }
  // Ecoute armee avant l'ecriture : l'evenement `config` peut arriver tout de suite.
  const wait = engineUp() ? waitConfigEvent(3000) : null;
  try {
    await save(json);
  } catch (e) {
    wait?.cancel();
    return { state: 'invalid', error: String(e), write: true }; // ecriture refusee : rien n'a change
  }
  const ev = wait && (await wait.promise);
  if (ev && !ev.ok) return { state: 'rejected', error: ev.error ?? 'refusé par le moteur' };
  const out = { state: ev ? 'saved' : 'saved-offline' };
  discard();
  return out;
}

// Quitter le brouillon (annuler, changer de profil, fermer) : demander seulement s'il est modifie.
export const guard = (dirty, action) => (dirty && action !== 'save' ? 'ask' : 'proceed');

export const modalOpen = () => !!document.querySelector('.nc-modal');

// Boite modale stylee nc-panel. buttons : [[valeur, libelle, principal?]] ; Echap = dernier bouton.
// Une seule a la fois : si une boite est deja ouverte, la reponse est le dernier bouton (« Rester »).
export function confirmBox(host, { title, text, buttons }) {
  const N = window.NullCtl;
  if (modalOpen()) return Promise.resolve(buttons.at(-1)[0]);
  return new Promise((resolve) => {
    const wrap = document.createElement('div');
    wrap.className = 'nc-modal';
    wrap.innerHTML = '<div class="nc-panel" role="dialog" aria-modal="true"><div class="nc-panel-head"><div><div class="nc-label"></div></div></div>' +
      '<div class="nc-section" style="padding-bottom:14px"><div class="nc-note"><i data-lucide="alert-triangle"></i><span></span></div></div><div class="nc-panel-foot"></div></div>';
    wrap.querySelector('.nc-label').textContent = title;
    wrap.querySelector('.nc-note span').textContent = text;
    const foot = wrap.querySelector('.nc-panel-foot');
    const done = (v) => { wrap.remove(); document.removeEventListener('keydown', key, true); resolve(v); };
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
}
