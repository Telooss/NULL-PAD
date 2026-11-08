// Mode configuration : sauvegarde (machine a etats pure), garde des changements non enregistres, boite de confirmation.

// deps : { json, validate(json), save(json), waitConfigEvent(ms) -> {ok,error}|null, engineUp() -> bool, discard() }
// -> { state: 'saved' | 'saved-offline' | 'invalid' | 'rejected', error? }. `discard` (sortie du mode,
// brouillon jete) n'est appele que pour saved / saved-offline : sinon le brouillon reste.
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
    return { state: 'invalid', error: String(e) }; // ecriture refusee (disque ou validation) : rien n'a change
  }
  const ev = wait && (await wait);
  if (ev && !ev.ok) return { state: 'rejected', error: ev.error ?? 'refusé par le moteur' };
  const out = { state: ev ? 'saved' : 'saved-offline' };
  discard();
  return out;
}

// Quitter le brouillon (annuler, changer de profil, fermer) : demander seulement s'il est modifie.
export const guard = (dirty, action) => (dirty && action !== 'save' ? 'ask' : 'proceed');

// Boite modale stylee nc-panel. buttons : [[valeur, libelle, principal?]] ; Echap = dernier bouton.
export function confirmBox(host, { title, text, buttons }) {
  const N = window.NullCtl;
  return new Promise((resolve) => {
    const wrap = document.createElement('div');
    wrap.className = 'nc-modal';
    wrap.innerHTML = '<div class="nc-panel" role="dialog" aria-modal="true"><div class="nc-panel-head"><div><div class="nc-label"></div></div></div>' +
      '<div class="nc-section" style="padding-bottom:14px"><div class="nc-note"><i data-lucide="alert-triangle"></i><span></span></div></div><div class="nc-panel-foot"></div></div>';
    wrap.querySelector('.nc-label').textContent = title;
    wrap.querySelector('.nc-note span').textContent = text;
    const foot = wrap.querySelector('.nc-panel-foot');
    const done = (v) => { wrap.remove(); document.removeEventListener('keydown', key, true); resolve(v); };
    const key = (e) => { if (e.key === 'Escape') { e.stopPropagation(); done(buttons.at(-1)[0]); } };
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
