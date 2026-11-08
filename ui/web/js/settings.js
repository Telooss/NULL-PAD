// Ecran Reglages : remplace la scene et le panneau. Brouillon propre (sound, obs, midi, timing) ;
// « Lancer avec Windows » n'est pas dans config.json : applique tout de suite.
import { h } from './editors/render.js';
import { guard, confirmBox, modalOpen } from './config-mode.js';
import { AUDIO } from './drop.js';
import { addSoundFile, clampPercent, clampPort, clampTiming, inUseMessage, isDirty, mergeSettings, normalize, settingsOf, TIMING_MAX, TIMING_MIN } from './settings-model.js';

const N = window.NullCtl;
const CABLE_URL = 'https://vb-audio.com/Cable/';
const TABS = [['audio', 'Audio'], ['sons', 'Sons'], ['obs', 'OBS'], ['midi', 'MIDI'], ['start', 'Démarrage']];
const MIC_WARN = 'Via l’appli, le micro passe par NULL/CONTROL : il ajoute un léger délai, et il est muet tant que le moteur n’est pas lancé.';
const icon = (n) => h('i', { 'data-lucide': n });
const field = (label, ...kids) => h('div', { class: 'nc-field' }, h('span', { class: 'nc-label' }, label), ...kids);
const note = (ic, ...kids) => h('div', { class: 'nc-note' }, icon(ic), h('span', {}, ...kids));
const LEAVE = { title: 'Changements non enregistrés', text: 'Les réglages ont des changements non enregistrés.',
  buttons: [['save', 'Enregistrer', true], ['discard', 'Abandonner'], ['stay', 'Rester']] };

// ctx : { main, app, invoke, pickFile, toast, config() -> config affichee, write(next) -> Promise<bool>, onClose() }
export function createSettings(ctx) {
  let root = null;
  let draft = null;
  let base = null;
  let tab = 'audio';
  let saving = false;
  let env = { outputs: [], cable: false, autostart: false };
  let msg = ''; // refus de retrait d'un son
  let probe = {}; // resultats des tests { obs, midi }

  const dirty = () => isDirty(draft, base);

  function slider(label, obj, key) {
    const out = h('output', {}, `${obj[key]} %`);
    const fill = h('div', { class: 'fill', style: `width:${obj[key]}%` });
    const knob = h('div', { class: 'knob', style: `left:${obj[key]}%` });
    const rng = h('input', { type: 'range', min: 0, max: 100, value: obj[key], 'aria-label': label,
      oninput: () => { const v = clampPercent(rng.value); obj[key] = v; out.textContent = `${v} %`; fill.style.width = `${v}%`; knob.style.left = `${v}%`; } });
    return field(label, h('div', { class: 'nc-slider' }, h('div', { class: 'track' }, fill, knob, rng), out));
  }
  const seg = (label, opts, value, set) => field(label, h('div', { class: 'nc-seg', role: 'group' }, opts.map(([v, l]) =>
    h('button', { class: v === value ? 'is-on' : '', 'aria-pressed': String(v === value), onclick: () => { set(v); render(); } }, l))));
  const text = (label, obj, key, props = {}) => field(label, h('input', { class: 'nc-input', value: obj[key] ?? '', spellcheck: false, autocomplete: 'off', oninput: (e) => { obj[key] = e.target.value; }, ...props }));
  const num = (label, obj, key, lo, hi, clamp) => {
    const inp = h('input', { class: 'nc-input', type: 'number', min: lo, max: hi, value: obj[key], onchange: () => { obj[key] = clamp(inp.value, obj[key]); inp.value = obj[key]; } });
    return field(`${label} (${lo} à ${hi})`, inp);
  };
  const deviceSelect = (label, obj, key, none) => {
    const cur = obj[key];
    const names = env.outputs.includes(cur) || !cur ? env.outputs : [cur, ...env.outputs];
    return field(label, h('select', { class: 'nc-input', 'aria-label': label, onchange: (e) => { obj[key] = e.target.value || null; } },
      h('option', { value: '', selected: !cur }, none),
      names.map((n) => h('option', { value: n, selected: n === cur }, env.outputs.includes(n) ? n : `${n} (absent)`))));
  };
  const tester = (name, cmd) => {
    const out = h('span', { class: 'nc-label', role: 'status' }, probe[name] ?? '');
    const b = h('button', { class: 'nc-btn', onclick: async () => {
      b.disabled = true;
      out.textContent = 'Test en cours…';
      try {
        const r = await ctx.invoke(cmd);
        probe[name] = r.ok ? 'Test réussi' : `Échec : ${r.error ?? 'refusé'}`;
      } catch (e) {
        probe[name] = `Échec : ${e}`;
      }
      out.textContent = probe[name];
      b.disabled = false;
    } }, icon('plug-zap'), 'Tester');
    return h('div', { class: 'nc-row' }, b, out);
  };

  const PANES = {
    audio: () => [
      deviceSelect('Sortie casque', draft.sound, 'monitor_device', 'Par défaut'),
      deviceSelect('Sortie VB-Cable', draft.sound, 'cable_device', 'Auto (VB-Cable)'),
      env.cable ? note('check', h('b', {}, 'VB-Cable détecté'))
        : note('alert-triangle', h('b', {}, 'VB-Cable absent'), ' : installez-le pour envoyer les sons et le micro dans vos appels. ',
          h('input', { class: 'nc-input nc-mono', readOnly: true, value: CABLE_URL, 'aria-label': 'Adresse de VB-Cable', onfocus: (e) => e.target.select() }),
          h('button', { class: 'nc-btn nc-btn-sm', title: 'Copier le lien', onclick: () => navigator.clipboard?.writeText(CABLE_URL).then(() => ctx.toast('Lien copié'), () => {}) }, icon('copy'))),
      slider('Volume général', draft.sound, 'master_volume'),
      slider('Volume casque', draft.sound, 'monitor_volume'),
      slider('Volume VB-Cable', draft.sound, 'cable_volume'),
      seg('Mode micro', [['External', 'Externe'], ['App', 'Via l’appli']], draft.sound.mic_mode, (v) => { draft.sound.mic_mode = v; }),
      draft.sound.mic_mode === 'App' ? note('alert-triangle', MIC_WARN) : null,
    ],
    sons: () => [
      msg ? note('alert-triangle', msg) : null,
      ...draft.sound.sounds.map((s) => h('div', { class: 'nc-step' },
        h('div', { class: 'nc-row' }, h('b', { class: 'nc-mono' }, s.id), h('span', { class: 'nc-label nc-ellipsis', title: s.path }, s.path),
          h('button', { class: 'nc-btn nc-btn-sm', title: `Retirer ${s.id}`, 'aria-label': `Retirer ${s.id}`, onclick: () => remove(s.id) }, icon('trash-2'))),
        slider('Volume', s, 'volume'),
        seg('Rappui', [['Restart', 'Relancer'], ['Stop', 'Couper']], s.on_repeat, (v) => { s.on_repeat = v; }))),
      draft.sound.sounds.length ? null : note('info', 'Aucun son dans la bibliothèque.'),
      h('div', { class: 'nc-row' }, h('button', { class: 'nc-btn', onclick: add }, icon('plus'), 'Ajouter un son'), h('span', { class: 'nc-label' }, 'ou glissez un fichier audio ici')),
    ],
    obs: () => [
      text('Hôte', draft.obs, 'host'),
      note('info', 'Gardez ', h('b', {}, '127.0.0.1'), ' si OBS tourne sur ce PC.'),
      num('Port', draft.obs, 'port', 1, 65535, clampPort),
      text('Mot de passe', draft.obs, 'password', { type: 'password', autocomplete: 'off' }),
      tester('obs', 'test_obs'),
      note('info', 'Le test utilise les réglages enregistrés.'),
    ],
    midi: () => [text('Port MIDI', draft.midi, 'port'), tester('midi', 'test_midi'), note('info', 'Le test envoie un CC avec les réglages enregistrés.')],
    start: () => {
      const toggle = () => setAutostart(!env.autostart);
      return [
        h('div', { class: `nc-check${env.autostart ? ' is-on' : ''}`, role: 'checkbox', tabIndex: 0, 'aria-checked': String(env.autostart), onclick: toggle,
          onkeydown: (e) => { if (e.key === ' ' || e.key === 'Enter') { e.preventDefault(); toggle(); } } },
        h('span', { class: 'box' }, env.autostart ? icon('check') : null), h('span', { class: 'tx' }, 'Lancer avec Windows')),
        note('info', 'Appliqué tout de suite, sans « Enregistrer ».'),
        num('Appui long (ms)', draft.timing, 'hold_ms', TIMING_MIN, TIMING_MAX, clampTiming),
        num('Double appui (ms)', draft.timing, 'double_ms', TIMING_MIN, TIMING_MAX, clampTiming),
      ];
    },
  };

  async function setAutostart(want) {
    try {
      await ctx.invoke('set_autostart', { enabled: want });
      env.autostart = want;
      ctx.toast(want ? 'Le moteur se lancera avec Windows' : 'Le moteur ne se lancera plus avec Windows');
    } catch (e) {
      ctx.toast(`Démarrage inchangé : ${e}`);
    }
    if (root) render();
  }

  async function add() {
    const p = await ctx.pickFile({ filters: [{ name: 'Audio', extensions: AUDIO }] });
    if (p && draft) { addSoundFile(draft, p); msg = ''; render(); }
  }
  function remove(id) {
    msg = inUseMessage(ctx.config(), id) ?? '';
    if (!msg) draft.sound.sounds = draft.sound.sounds.filter((s) => s.id !== id);
    render();
  }

  function render() {
    const pane = root.querySelector('.nc-set-pane');
    const top = pane.scrollTop;
    root.querySelectorAll('[role=tab]').forEach((b) => { const on = b.dataset.tab === tab; b.classList.toggle('is-on', on); b.setAttribute('aria-selected', String(on)); });
    pane.replaceChildren(h('div', { class: 'nc-section' }, ...PANES[tab]().filter(Boolean)));
    pane.scrollTop = top;
    N.icons();
  }

  async function save() {
    if (saving) return false;
    saving = true;
    root.inert = true;
    try {
      let latest;
      try {
        latest = await ctx.invoke('get_config'); // relue juste avant : les profils changes entre-temps sont gardes
      } catch (e) {
        ctx.toast(`Config illisible : ${e}`);
        return false;
      }
      if (!(await ctx.write(mergeSettings(latest, normalize(draft))))) return false;
      ctx.toast('Réglages enregistrés');
      close();
      return true;
    } finally {
      saving = false;
      if (root) root.inert = false;
    }
  }

  const onKey = (e) => { if (e.key === 'Escape' && !modalOpen() && !saving) leave(); };

  function close() {
    document.removeEventListener('keydown', onKey);
    root?.remove();
    root = draft = base = null;
    ctx.onClose();
  }

  // Quitter : demander seulement si modifie. true si l'ecran est ferme.
  async function leave() {
    if (!root) return true;
    if (guard(dirty(), 'leave') === 'proceed') { close(); return true; }
    const c = await confirmBox(ctx.app, LEAVE);
    if (!root) return true;
    if (c === 'save') return save();
    if (c === 'discard') { close(); return true; }
    return false;
  }

  async function open() {
    if (root) return;
    base = settingsOf(ctx.config());
    draft = structuredClone(base);
    tab = 'audio';
    msg = '';
    probe = {};
    root = h('section', { class: 'nc-panel nc-settings', 'aria-label': 'Réglages' },
      h('div', { class: 'nc-panel-head' }, h('div', { class: 'nc-label' }, 'Réglages'), h('button', { class: 'nc-btn nc-btn-sm', title: 'Fermer (Échap)', 'aria-label': 'Fermer', onclick: leave }, icon('x'))),
      h('div', { class: 'nc-set-body' },
        h('div', { class: 'nc-tabs', role: 'tablist', 'aria-orientation': 'vertical' }, TABS.map(([k, l]) =>
          h('button', { role: 'tab', 'data-tab': k, onclick: () => { tab = k; render(); } }, l))),
        h('div', { class: 'nc-set-pane' })),
      h('div', { class: 'nc-panel-foot' }, h('button', { class: 'nc-btn', onclick: leave }, 'Annuler'),
        h('button', { class: 'nc-btn is-primary', onclick: save }, icon('save'), 'Enregistrer')));
    ctx.main.append(root);
    document.addEventListener('keydown', onKey);
    render();
    const [outputs, cable, autostart] = await Promise.all([
      ctx.invoke('list_audio_outputs').catch(() => []), ctx.invoke('cable_present').catch(() => false), ctx.invoke('get_autostart').catch(() => false)]);
    if (root) { env = { outputs, cable, autostart }; render(); }
  }

  // Fichiers deposes : les sons audio rejoignent la bibliotheque.
  function drop(paths = []) {
    if (!root || saving) return;
    const ok = paths.filter((p) => AUDIO.includes(p.split('.').pop().toLowerCase()));
    if (!ok.length) return ctx.toast('Déposer un fichier audio (mp3, wav, ogg, flac)');
    ok.forEach((p) => addSoundFile(draft, p));
    tab = 'sons';
    msg = '';
    render();
  }

  return { open, leave, drop, isOpen: () => !!root };
}
