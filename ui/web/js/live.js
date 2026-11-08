// Evenements du moteur en direct : appuis, actions, profil, lien, etat du moteur et de l'enregistrement.
import { toastText } from './view.js';

const N = window.NullCtl;

// Une ligne dans la scene, 4 s (nc-toast du design system ; la nouvelle remplace l'ancienne).
export function toast(app, text) {
  app.querySelectorAll('.nc-toast').forEach((n) => n.remove());
  const t = document.createElement('div');
  t.className = 'nc-toast';
  t.innerHTML = '<i data-lucide="activity"></i><span></span>';
  t.lastChild.textContent = text;
  app.appendChild(t);
  N.icons();
  setTimeout(() => t.remove(), 4000);
}

// ctx : { invoke, listen, app, pad, shell, binding(input, trigger, profile), setProfile(id, glitch), setLink(link), onConfig(e) }
export async function startLive({ invoke, listen, app, pad, shell, binding, setProfile, setLink, onConfig, onUp }) {
  async function probe(follow) {
    try {
      const s = await invoke('engine_status');
      shell.setEngine(true);
      setLink(s.link);
      if (follow) setProfile(s.profile, false);
    } catch {
      shell.setEngine(false);
      setLink('none');
    }
  }

  await listen('engine-event', ({ payload: e }) => {
    switch (e.type) {
      case 'key': pad.press(e.input, e.down); break;
      case 'action': toast(app, toastText(e, binding(e.input, e.trigger, e.profile))); break;
      case 'profile': setProfile(e.profile, true); break;
      case 'link': setLink(e.link); break;
      case 'config': shell.setSaved(e.ok, e.error); onConfig(e); break;
    }
  });
  await listen('engine-up', () => { probe(true); onUp?.(); });
  await listen('engine-down', () => { shell.setEngine(false); setLink('none'); });
  shell.onStart(() => invoke('start_engine').catch((err) => toast(app, String(err))));
  await probe(true);
}
