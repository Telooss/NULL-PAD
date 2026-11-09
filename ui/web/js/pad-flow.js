// Pad (ZMK Studio) cote interface : pastille, keymap connue, lecture avec deverrouillage M1 + M3, ecriture pour saveFlow.
// Le metier pur est dans pad-model.js ; ici, les commandes Tauri et les boites de dialogue.
import { confirmBox, modalOpen } from './config-mode.js';
import { parseErr, pillState, changedProfiles, retryBusy } from './pad-model.js';

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

// ctx : { invoke, listen, app, shell, toast(text), onModel(model) } — onModel : la keymap connue a change (legendes).
export function createPadFlow({ invoke, listen, app, shell, toast, onModel }) {
  let model = null; // derniere keymap connue (cache, lecture ou ecriture)
  let status = null; // dernier pad_status
  const padBehind = new Set(); // profils dont la couche n'a pas suivi une operation faite sans le pad
  let configBehind = false; // pad ecrit, config pas enregistree ensuite
  let pending = 0; // commandes du pad en cours
  let polling = null; // pad_status en cours : il tient le port cote Rust
  let unlock = null; // boite « Appuie sur M1 + M3 » ouverte

  const pill = () => shell.setPad(pillState(status, padBehind.size > 0 || configBehind));
  const setModel = (m) => { model = m; onModel(m); };

  // Rust attend M1 + M3 : boite annulable, fermee quand la commande se termine.
  listen('pad-unlock-needed', () => {
    if (!pending || unlock || modalOpen()) return;
    unlock = confirmBox(app, { title: 'Pad verrouillé', text: 'Appuie sur M1 + M3 pour déverrouiller le pad.', buttons: [['cancel', 'Annuler']] });
    unlock.then((v) => { unlock = null; if (v === 'cancel') invoke('pad_cancel_unlock').catch(() => {}); });
  });

  // Attend la sonde en cours (sinon `busy`), puis un nouvel essai si le port est encore tenu.
  async function run(cmd, args) {
    pending++;
    try {
      await polling;
      return await retryBusy(() => invoke(cmd, args), () => sleep(250));
    } finally {
      if (--pending === 0) unlock?.close('done');
    }
  }

  function poll() {
    if (pending || polling || modalOpen()) return polling;
    polling = (async () => {
      try {
        const s = await invoke('pad_status');
        if (!s.busy || !status) status = s; // occupe : la pastille ne change pas
      } catch { /* pad_status ne rend pas d'erreur */ }
      polling = null;
      pill();
    })();
    return polling;
  }

  // Changements laisses ailleurs (ZMK Studio) : abandonnes seulement si l'utilisateur le confirme.
  async function discardForeign(no) {
    const c = await confirmBox(app, { title: 'Changements dans le pad', text: 'Le pad a des changements non enregistrés faits ailleurs. Les abandonner ?',
      buttons: [['discard', 'Abandonner les changements du pad'], ['no', no, true]] });
    if (c !== 'discard') return false;
    await run('pad_discard_foreign');
    return true;
  }

  // Lecture complete -> { model, max_name_len } ou null (toast : « Pad non lu … : <why> »). no : bouton qui refuse l'abandon.
  async function read(why = 'touches natives en lecture seule', no = 'Continuer sans le pad') {
    for (let retried = false; ; retried = true) {
      try {
        const r = await run('pad_read', { waitUnlockSecs: 60 });
        setModel(r.model);
        status = { present: true, locked: false, busy: false };
        pill();
        return r;
      } catch (e) {
        const err = parseErr(e);
        try {
          if (err.code === 'foreign-unsaved' && !retried && (await discardForeign(no))) continue;
        } catch (e2) {
          toast(`Pad non lu : ${parseErr(e2).message}`);
          return null;
        }
        toast(err.code === 'foreign-unsaved' ? `Pad non lu : ${why}` : `Pad non lu (${err.message}) : ${why}`);
        poll();
        return null;
      }
    }
  }

  return {
    model: () => model,
    async init() {
      const c = await invoke('pad_cached').catch(() => null);
      if (c) setModel(c.model);
      await poll();
      setInterval(poll, 5000);
    },
    // Pad branche : lecture (avec deverrouillage) -> { expected, target, maxLen } ;
    // null : pas de pad ; false : pad la mais occupe, refuse ou annule (toast « … : <why> » deja affiche).
    async edit(why, no) {
      await polling;
      let s = await invoke('pad_status').catch(() => null);
      if (s?.busy) { await sleep(250); s = await invoke('pad_status').catch(() => null); }
      if (!s?.present) return null;
      if (s.busy) { toast(`Pad occupé : ${why ?? 'touches natives en lecture seule'}`); return false; }
      const r = await read(why, no);
      return r ? { expected: r.model, target: structuredClone(r.model), maxLen: r.max_name_len } : false;
    },
    // Dependance `pad` de saveFlow.
    deps: (expected, target) => ({ expected, target,
      write: async (e, t) => {
        const m = (await run('pad_write', { expected: e, target: t })).model; // PadRead -> PadModel
        for (const p of changedProfiles(e, m)) padBehind.delete(p); // couches reecrites : a jour
        return m;
      },
      onForeign: () => discardForeign('Annuler') }),
    // Apres saveFlow : keymap ecrite connue ; config non ecrite apres le pad -> « Pad pas à jour » jusqu'au prochain enregistrement.
    settle(r) {
      if (r.padModel) setModel(r.padModel);
      if (r.state === 'saved' || r.state === 'saved-offline') configBehind = false;
      else if (r.padUpdated && r.state === 'invalid') configBehind = true;
      pill();
    },
    // Operation sur le profil p faite sans le pad : sa couche n'a pas suivi.
    markStale(p) { padBehind.add(p); pill(); },
    // Pad refuse (rien n'a ete ecrit) : modifie ailleurs -> proposer de relire ; sinon un message.
    // -> nouvelle lecture { model, max_name_len } si l'utilisateur a recharge, sinon null.
    async onPadError(r) {
      if (r.code !== 'changed-elsewhere') {
        toast(`Pad non écrit, rien n’a été enregistré : ${r.error}`);
        return null;
      }
      const c = await confirmBox(app, { title: 'Pad modifié ailleurs', text: 'Le pad a changé depuis sa lecture. Recharger perd les changements du pad ; le brouillon de la config est gardé.',
        buttons: [['reload', 'Recharger', true], ['stay', 'Rester']] });
      return c === 'reload' ? read() : null;
    },
  };
}
