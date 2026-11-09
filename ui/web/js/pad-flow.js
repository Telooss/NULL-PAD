// Pad (ZMK Studio) cote interface : pastille, keymap connue, lecture avec deverrouillage M1 + M3, ecriture pour saveFlow.
// Le metier pur est dans pad-model.js ; ici, les commandes Tauri et les boites de dialogue.
import { confirmBox, modalOpen } from './config-mode.js';
import { parseErr, pillState } from './pad-model.js';

// ctx : { invoke, listen, app, shell, toast(text), onModel(model) } — onModel : la keymap connue a change (legendes).
export function createPadFlow({ invoke, listen, app, shell, toast, onModel }) {
  let model = null; // derniere keymap connue (cache, lecture ou ecriture)
  let status = null; // dernier pad_status
  let stale = false; // une operation sur les profils ou la config n'a pas atteint le pad
  let pending = 0; // commandes du pad en cours
  let unlock = null; // boite « Appuie sur M1 + M3 » ouverte

  const pill = () => shell.setPad(pillState(status, stale));
  const setModel = (m) => { model = m; onModel(m); };

  // Rust attend M1 + M3 : boite annulable, fermee quand la commande se termine.
  listen('pad-unlock-needed', () => {
    if (!pending || unlock || modalOpen()) return;
    unlock = confirmBox(app, { title: 'Pad verrouillé', text: 'Appuie sur M1 + M3 pour déverrouiller le pad.', buttons: [['cancel', 'Annuler']] });
    unlock.then((v) => { unlock = null; if (v === 'cancel') invoke('pad_cancel_unlock').catch(() => {}); });
  });

  async function run(cmd, args) {
    pending++;
    try {
      return await invoke(cmd, args);
    } finally {
      if (--pending === 0) unlock?.close('done');
    }
  }

  async function poll() {
    if (pending || modalOpen()) return;
    try {
      const s = await invoke('pad_status');
      if (!s.busy || !status) status = s; // occupe : la pastille ne change pas
    } catch { /* pad_status ne rend pas d'erreur */ }
    pill();
  }

  // Changements laisses ailleurs (ZMK Studio) : abandonnes seulement si l'utilisateur le confirme.
  async function discardForeign(no) {
    const c = await confirmBox(app, { title: 'Changements dans le pad', text: 'Le pad a des changements non enregistrés faits ailleurs. Les abandonner ?',
      buttons: [['discard', 'Abandonner les changements du pad'], ['no', no, true]] });
    if (c !== 'discard') return false;
    await run('pad_discard_foreign');
    return true;
  }

  // Lecture complete -> { model, max_name_len } ou null (toast : on continue sans editer les natives).
  async function read() {
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
          if (err.code === 'foreign-unsaved' && !retried && (await discardForeign('Continuer sans le pad'))) continue;
        } catch (e2) {
          toast(`Pad non lu : ${parseErr(e2).message}`);
          return null;
        }
        toast(err.code === 'foreign-unsaved' ? 'Pad non lu : touches natives en lecture seule' : `Pad non lu (${err.message}) : touches natives en lecture seule`);
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
    // Pad branche et libre : lecture (avec deverrouillage) -> { expected, target, maxLen } ; sinon null.
    async edit() {
      const s = await invoke('pad_status').catch(() => null);
      if (!s?.present || s.busy) return null;
      const r = await read();
      return r && { expected: r.model, target: structuredClone(r.model), maxLen: r.max_name_len };
    },
    // Dependance `pad` de saveFlow.
    deps: (expected, target) => ({ expected, target,
      write: async (e, t) => (await run('pad_write', { expected: e, target: t })).model, // PadRead -> PadModel
      onForeign: () => discardForeign('Annuler') }),
    // Apres saveFlow : keymap ecrite connue ; config non ecrite apres le pad -> « Pad pas à jour ».
    settle(r) {
      if (!r.padModel) return;
      setModel(r.padModel);
      stale = r.state === 'invalid';
      pill();
    },
    markStale() { stale = true; pill(); },
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
