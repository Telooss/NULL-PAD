// Pont vers le cote Rust. Sans window.__TAURI__ (navigateur ordinaire), un mock sert une config d'exemple.
const T = window.__TAURI__;

const key = (input, action, label, trigger = 'Press') => ({ input, trigger, action, ...(label ? { label } : {}) });
const sample = () => ({
  version: 1,
  timing: { hold_ms: 300, double_ms: 250 },
  profiles: [
    { id: 1, name: 'Default', tag: 'DEF', bindings: [
      key('K04', { type: 'Shortcut', keys: 'Ctrl+C' }, 'Copier'),
      key('K05', { type: 'Shortcut', keys: 'Ctrl+V' }, 'Coller'),
      key('K06', { type: 'Text', text: 'bonjour' }),
    ] },
    { id: 2, name: 'Stream', tag: 'STR', bindings: [
      key('K00', { type: 'ObsScene', scene: 'Jeu' }, 'Scène jeu'),
      key('K01', { type: 'Sound', id: 'airhorn' }, 'Airhorn'),
      key('K02', { type: 'ObsRecord', op: 'Toggle' }),
      key('ENC_L_CW', { type: 'Volume', delta: 2 }),
    ] },
  ],
  sound: { master_volume: 100, monitor_device: null, monitor_volume: 100, cable_device: null, cable_volume: 100, mic_mode: 'External', mic_device: null, mic_gain: 100,
    sounds: [{ id: 'airhorn', path: 'C:\Sons\airhorn.wav', volume: 100, on_repeat: 'Restart' }, { id: 'clap', path: 'C:\Sons\clap.mp3', volume: 80, on_repeat: 'Stop' }] },
  obs: { host: '127.0.0.1', port: 4455, password: null },
  midi: { port: 'NULL-PAD' },
});

let mockCfg = sample();
const listeners = new Map();
const emit = (evt, payload) => (listeners.get(evt) || []).forEach((cb) => cb({ event: evt, payload }));

const mock = {
  get_config: () => structuredClone(mockCfg),
  save_config: ({ config }) => {
    if (!config || config.version !== 1 || !Array.isArray(config.profiles)) throw 'config invalide';
    mockCfg = structuredClone(config);
    return 'ok';
  },
  status: () => ({ engine: true, profile: 1 }),
};

if (!T && new URLSearchParams(location.search).get('demo') === '1') {
  let n = 0;
  setInterval(() => emit('engine-event', { type: 'key', profile: 1, input: ['K04', 'K05', 'K06'][n++ % 3] }), 3000);
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
