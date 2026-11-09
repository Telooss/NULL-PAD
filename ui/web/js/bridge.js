// Pont vers le cote Rust. Sans window.__TAURI__ (navigateur ordinaire), un mock sert une config d'exemple.
const T = window.__TAURI__;

const key = (input, action, label, trigger = 'Press') => ({ input, trigger, action, ...(label ? { label } : {}) });
const sample = () => ({
  version: 1,
  timing: { hold_ms: 300, double_ms: 250 },
  profiles: [
    // Profil 1 : la grille est native (firmware), seules K11 et K19 portent des liaisons.
    { id: 1, name: 'Default', tag: 'DEF', bindings: [
      key('K11', { type: 'Shortcut', keys: 'Ctrl+C' }, 'Copier'),
      key('K19', { type: 'Shortcut', keys: 'Ctrl+V' }, 'Coller'),
    ] },
    { id: 2, name: 'Stream', tag: 'STR', bindings: [
      key('K00', { type: 'ObsScene', scene: 'Jeu' }, 'Scène jeu'),
      key('K01', { type: 'Sound', id: 'airhorn' }, 'Airhorn'),
      key('K02', { type: 'Sound', id: 'clap' }),
      key('K03', { type: 'MidiNote', channel: 1, note: 60, velocity: 100 }),
      key('K04', { type: 'ObsRecord', op: 'Toggle' }),
      key('K04', { type: 'ObsReplaySave' }, undefined, 'Hold'),
      key('K05', { type: 'Toggle', on: { type: 'MicMute' }, off: { type: 'VolumeMute' } }, 'Micro'),
      key('ENC_R_CW', { type: 'Volume', delta: 2 }),
      key('ENC_R_CCW', { type: 'Volume', delta: -2 }),
      key('ENC_R_PRESS', { type: 'MicMute' }),
    ] },
  ],
  sound: { master_volume: 100, monitor_device: null, monitor_volume: 100, cable_device: null, cable_volume: 100, mic_mode: 'External', mic_device: null, mic_gain: 100,
    sounds: [{ id: 'airhorn', path: 'C:\\Sons\\airhorn.wav', volume: 100, on_repeat: 'Restart' }, { id: 'clap', path: 'C:\\Sons\\clap.mp3', volume: 80, on_repeat: 'Stop' }] },
  obs: { host: '127.0.0.1', port: 4455, password: null },
  midi: { port: 'NULL-PAD' },
});

let mockCfg = sample();
let mockProfile = 1;
let mockAutostart = false;
const listeners = new Map();
const emit = (evt, payload) => (listeners.get(evt) || []).forEach((cb) => cb({ event: evt, payload }));

const mock = {
  get_config: () => structuredClone(mockCfg),
  // Le vrai controle est engine::config::parse cote Rust ; le mock ne verifie que la forme et les sons.
  validate_config: ({ json }) => {
    const c = JSON.parse(json);
    if (c.version !== 1 || !Array.isArray(c.profiles)) throw 'config invalide';
    const ids = new Set((c.sound?.sounds ?? []).map((x) => x.id));
    const miss = [...JSON.stringify(c.profiles).matchAll(/"type":"Sound","id":"([^"]*)"/g)].map((m) => m[1]).find((id) => !ids.has(id));
    if (miss !== undefined) throw `config invalide: son "${miss}" absent de sound.sounds`;
  },
  save_config: ({ json, pad }) => {
    mock.validate_config({ json, pad });
    mockCfg = JSON.parse(json);
    setTimeout(() => emit('engine-event', { type: 'config', ok: true, error: null }), 200);
  },
  engine_status: () => ({ profile: mockProfile, link: 'usb', version: '0.1.0' }),
  start_engine: () => null,
  list_audio_outputs: () => ['Speakers (Realtek Audio)', 'Headphones (Focusrite USB Audio)', 'CABLE Input (VB-Audio Virtual Cable)'],
  cable_present: () => true,
  get_autostart: () => mockAutostart,
  set_autostart: ({ enabled }) => { mockAutostart = enabled; },
  test_obs: () => ({ ok: false, error: 'OBS injoignable (127.0.0.1:4455)' }),
  test_midi: () => ({ ok: true }),
  // Legendes de la couche 0 du vrai keymap (K11, K19 : actions).
  native_keys: () => ({ K00: 'VERR NUM', K01: '/', K02: '*', K03: '−', K04: '7', K05: '8', K06: '9', K07: '+', K08: '4', K09: '5', K10: '6',
    K12: '1', K13: '2', K14: '3', K15: 'ENTRÉE', K16: '0', K17: '00', K18: '.', M1: 'P1', M2: 'P2', M3: 'P3',
    ENC_L_CW: 'VOL+', ENC_L_CCW: 'VOL−', ENC_L_PRESS: 'MUET' }),
};

// ?pad=1 : faux pad (ZMK Studio). &padlock=1 : verrouille, deverrouille 2 s apres la demande ;
// &paderr=changed-elsewhere|foreign-unsaved|disconnected : pad_write echoue une fois ; ?pad=absent : pas de pad.
const qs = new URLSearchParams(location.search);
if (!T && qs.get('pad')) {
  const absent = qs.get('pad') === 'absent';
  let locked = qs.get('padlock') === '1';
  let err = qs.get('paderr');
  let cancel = false;
  const kp = (id, label) => ({ kind: 'native', usage: (7 << 16) | id, label });
  const cc = (id, label) => ({ kind: 'native', usage: (0x0c << 16) | id, label });
  const defLayer = (p) => {
    const act = (i) => ({ kind: 'action', profile: p, input: i });
    const grid = p > 1 ? Array.from({ length: 20 }, (_, i) => act(i)) : [kp(0x53, 'Verr Num'), kp(0x54, '/'), kp(0x55, '*'), kp(0x56, '-'),
      kp(0x5f, '7'), kp(0x60, '8'), kp(0x61, '9'), kp(0x57, '+'), kp(0x5c, '4'), kp(0x5d, '5'), kp(0x5e, '6'), act(11),
      kp(0x59, '1'), kp(0x5a, '2'), kp(0x5b, '3'), kp(0x58, 'Entrée'), kp(0x62, '0'), { kind: 'other', behavior: 'np_00', p1: 0, p2: 0 }, kp(0x63, '.'), act(19)];
    return [...grid, ...[1, 2, 3].map((n) => ({ kind: 'profile_macro', n })), cc(0xe9, 'Volume +'), cc(0xea, 'Volume −'), cc(0xe2, 'Muet'), act(26), act(27), act(28)];
  };
  let km = { layers: Array.from({ length: 8 }, (_, i) => ({ id: i, name: i ? `P0${i + 1}` : 'DEF', bindings: defLayer(i + 1) })) };
  const read = () => ({ model: structuredClone(km), max_name_len: 16 });
  const present = () => { if (absent) throw 'absent: Pad non branché'; };
  const unlock = async () => {
    if (!locked) return;
    cancel = false;
    emit('pad-unlock-needed', {});
    for (let t = 0; t < 20; t++) {
      await new Promise((r) => setTimeout(r, 100));
      if (cancel) throw 'cancelled: Déverrouillage annulé';
    }
    locked = false;
    emit('pad-unlocked', {});
  };
  const letters = Array.from({ length: 26 }, (_, i) => ({ usage: (7 << 16) | (4 + i), label: String.fromCharCode(65 + i), group: 'Clavier' }));
  const digits = Array.from({ length: 10 }, (_, i) => ({ usage: (7 << 16) | (0x1e + i), label: String((i + 1) % 10), group: 'Clavier' }));
  const pad = [...Array.from({ length: 9 }, (_, i) => kp(0x59 + i, String(i + 1))), kp(0x62, '0'), kp(0x58, 'Entrée'), kp(0x57, '+')].map((b) => ({ usage: b.usage, label: b.label, group: 'Pavé numérique' }));
  const media = [cc(0xe9, 'Volume +'), cc(0xea, 'Volume −'), cc(0xe2, 'Muet'), cc(0xcd, 'Lecture/Pause')].map((b) => ({ usage: b.usage, label: b.label, group: 'Média et volume' }));
  Object.assign(mock, {
    pad_status: () => ({ present: !absent, locked: null, serial: null, busy: false }), // enumeration seulement, comme Rust
    pad_cached: () => read(),
    pad_cancel_unlock: () => { cancel = true; },
    pad_read: async () => { present(); await unlock(); return read(); },
    pad_write: async ({ expected, target }) => {
      present();
      await unlock();
      if (err) { const e = err; err = null; throw `${e}: erreur simulée (${e})`; }
      if (JSON.stringify(expected) !== JSON.stringify(km)) throw 'changed-elsewhere: Le pad a changé depuis la lecture';
      const bad = target.layers.find((l) => !l.name || l.name.length > 16 || !l.bindings.some((b) => b.kind === 'profile_macro' && b.n === 1));
      if (bad) throw `guard: La couche « ${bad.name} » doit garder une touche np_p1`;
      km = structuredClone(target);
      return read();
    },
    pad_discard_foreign: () => null,
    hid_catalog: () => [...letters, ...digits, ...pad, ...media],
    pad_default_layer: ({ profile }) => defLayer(profile),
  });
  // Avec une keymap : aucune liaison sur une position native (store::validate cote Rust).
  const validate = mock.validate_config;
  mock.validate_config = ({ json, pad: m }) => {
    validate({ json });
    const I = ['M1', 'M2', 'M3', 'ENC_L_CW', 'ENC_L_CCW', 'ENC_L_PRESS', 'ENC_R_CW', 'ENC_R_CCW', 'ENC_R_PRESS'];
    const idx = (n) => (/^K\d\d$/.test(n) ? +n.slice(1) : 20 + I.indexOf(n));
    for (const p of m ? JSON.parse(json).profiles : []) {
      const b = p.bindings.find((x) => { const k = m.layers[p.id - 1]?.bindings[idx(x.input)]; return !(k?.kind === 'action' && k.profile === p.id && k.input === idx(x.input)); });
      if (b) throw `touche native dans le pad : ${b.input} (profil ${p.id}), repasse-la en action pour la lier`;
    }
  };
}

// ?demo=1 : appuis, actions et changements de profil simules.
if (!T && new URLSearchParams(location.search).get('demo') === '1') {
  const keys = { 1: ['K11', 'K19'], 2: ['K00', 'K01', 'K02', 'K05'] };
  let n = 0;
  setInterval(() => {
    const ks = keys[mockProfile];
    const input = ks[n++ % ks.length];
    emit('engine-event', { type: 'key', profile: mockProfile, input, down: true });
    const b = mockCfg.profiles.find((p) => p.id === mockProfile)?.bindings.find((x) => x.input === input);
    const bad = n % 4 === 0;
    emit('engine-event', { type: 'action', input, trigger: 'Press', kind: b?.action.type ?? 'Text', ok: !bad, error: bad ? 'OBS injoignable' : null });
    setTimeout(() => emit('engine-event', { type: 'key', profile: mockProfile, input, down: false }), 150);
  }, 3000);
  setInterval(() => {
    mockProfile = mockProfile === 1 ? 2 : 1;
    emit('engine-event', { type: 'profile', profile: mockProfile });
  }, 15000);
}

export async function invoke(cmd, args = {}) {
  if (T) return T.core.invoke(cmd, args);
  if (!mock[cmd]) throw `commande inconnue: ${cmd}`;
  return mock[cmd](args);
}

export async function listen(evt, cb) {
  if (T) return T.event.listen(evt, cb);
  if (!listeners.has(evt)) listeners.set(evt, []);
  listeners.get(evt).push(cb);
  return () => listeners.set(evt, listeners.get(evt).filter((f) => f !== cb));
}

export const isMock = !T;

// Depot de fichiers natif (Tauri : position en pixels physiques, relative a la webview).
// cb({ type: 'over' | 'drop' | 'leave', paths?, x, y }) en pixels CSS (clientX/Y).
export function onFileDrop(cb) {
  if (T) {
    return T.webview.getCurrentWebview().onDragDropEvent(({ payload: p }) => {
      const r = devicePixelRatio || 1;
      cb({ type: p.type === 'enter' ? 'over' : p.type, paths: p.paths, x: (p.position?.x ?? 0) / r, y: (p.position?.y ?? 0) / r });
    });
  }
  // Mock : __simulateDrop(paths, x, y), et un vrai depot HTML5 (noms de fichiers seulement) au niveau du document.
  window.__simulateDrop = (paths, x, y) => { cb({ type: 'over', paths, x, y }); cb({ type: 'drop', paths, x, y }); };
  document.addEventListener('dragover', (e) => { e.preventDefault(); cb({ type: 'over', x: e.clientX, y: e.clientY }); });
  document.addEventListener('dragleave', (e) => { if (!e.relatedTarget) cb({ type: 'leave' }); });
  document.addEventListener('drop', (e) => {
    e.preventDefault();
    cb({ type: 'drop', paths: [...(e.dataTransfer?.files ?? [])].map((f) => f.name), x: e.clientX, y: e.clientY });
  });
  return Promise.resolve(() => {});
}

// Selecteur de fichier Windows (tauri-plugin-dialog) ; null si annule.
export async function pickFile({ filters = [] } = {}) {
  if (T) return T.dialog.open({ multiple: false, directory: false, filters });
  const audio = filters.some((f) => f.extensions.includes('mp3'));
  return audio ? 'C:\\Sons\\nouveau.wav' : 'C:\\Programmes\\appli.exe';
}

// Fermeture de la fenetre : ask() -> true pour fermer (booleen ou promesse).
export function onCloseRequested(ask) {
  if (T) {
    return T.window.getCurrentWindow().onCloseRequested(async (ev) => {
      if (!(await ask())) ev.preventDefault();
    });
  }
  // Mock : seul le dialogue natif du navigateur est possible.
  window.addEventListener('beforeunload', (e) => {
    if (ask() !== true) { e.preventDefault(); e.returnValue = ''; }
  });
  return Promise.resolve(() => {});
}
