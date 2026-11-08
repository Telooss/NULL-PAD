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
const listeners = new Map();
const emit = (evt, payload) => (listeners.get(evt) || []).forEach((cb) => cb({ event: evt, payload }));

const mock = {
  get_config: () => structuredClone(mockCfg),
  save_config: ({ config }) => {
    if (!config || config.version !== 1 || !Array.isArray(config.profiles)) throw 'config invalide';
    mockCfg = structuredClone(config);
    return 'ok';
  },
  engine_status: () => ({ profile: mockProfile, link: 'usb', version: '0.1.0' }),
  start_engine: () => null,
  // Legendes de la couche 0 du vrai keymap (K11, K19 : actions).
  native_keys: () => ({ K00: 'VERR NUM', K01: '/', K02: '*', K03: '−', K04: '7', K05: '8', K06: '9', K07: '+', K08: '4', K09: '5', K10: '6',
    K12: '1', K13: '2', K14: '3', K15: 'ENTRÉE', K16: '0', K17: '00', K18: '.', M1: 'P1', M2: 'P2', M3: 'P3',
    ENC_L_CW: 'VOL+', ENC_L_CCW: 'VOL−', ENC_L_PRESS: 'MUET' }),
};

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
