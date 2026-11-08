import { test } from 'node:test';
import assert from 'node:assert/strict';
import { soundInUse, inUseMessage, mergeSettings, settingsOf, normalize, isDirty, clampTiming, clampPort, addSoundFile } from '../js/settings-model.js';

const key = (input, action, trigger = 'Press') => ({ input, trigger, action });
const cfg = () => ({
  version: 1, timing: { hold_ms: 300, double_ms: 250 },
  profiles: [
    { id: 1, name: 'Default', tag: 'DEF', bindings: [key('K11', { type: 'Shortcut', keys: 'Ctrl+C' })] },
    { id: 2, name: 'Stream', tag: 'STR', bindings: [
      key('K01', { type: 'Sound', id: 'airhorn' }),
      key('K05', { type: 'Toggle', on: { type: 'Sound', id: 'clap' }, off: { type: 'MicMute' } }),
      key('K06', { type: 'Macro', steps: [{ type: 'Do', action: { type: 'Sound', id: 'clap' } }, { type: 'Wait', ms: 5 }] }),
    ] },
  ],
  sound: { master_volume: 100, monitor_device: null, monitor_volume: 100, cable_device: null, cable_volume: 100, mic_mode: 'External', mic_device: null, mic_gain: 100,
    sounds: [{ id: 'airhorn', path: 'a.wav', volume: 100, on_repeat: 'Restart' }, { id: 'clap', path: 'c.mp3', volume: 80, on_repeat: 'Stop' }, { id: 'free', path: 'f.wav', volume: 100, on_repeat: 'Restart' }] },
  obs: { host: '127.0.0.1', port: 4455, password: null },
  midi: { port: 'NULL-PAD' },
});

test('soundInUse finds direct, toggle and macro uses', () => {
  assert.deepEqual(soundInUse(cfg(), 'airhorn'), [{ profile: 'Stream', input: 'K01' }]);
  assert.deepEqual(soundInUse(cfg(), 'clap').map((u) => u.input), ['K05', 'K06']);
  assert.deepEqual(soundInUse(cfg(), 'free'), []);
  assert.equal(inUseMessage(cfg(), 'free'), null);
  assert.match(inUseMessage(cfg(), 'clap'), /K05 \(Stream\), K06 \(Stream\)/);
});

test('mergeSettings replaces only the 4 sections and keeps newer profiles', () => {
  const draft = settingsOf(cfg());
  draft.obs.host = 'pc';
  draft.timing.hold_ms = 500;
  const latest = cfg();
  latest.profiles[0].name = 'Renomme'; // modifie entre-temps
  latest.obs.host = 'ancien';
  const m = mergeSettings(latest, draft);
  assert.equal(m.profiles[0].name, 'Renomme');
  assert.equal(m.obs.host, 'pc');
  assert.equal(m.timing.hold_ms, 500);
  assert.equal(m.version, 1);
  assert.equal(latest.obs.host, 'ancien', 'entree non modifiee');
});

test('bounds and normalize', () => {
  assert.equal(clampTiming(10), 50);
  assert.equal(clampTiming(99999), 5000);
  assert.equal(clampTiming('abc', 300), 300);
  assert.equal(clampTiming(120.4), 120);
  assert.equal(clampPort(0), 1);
  assert.equal(clampPort(70000), 65535);
  const d = settingsOf(cfg());
  d.obs.password = '';
  d.timing.hold_ms = 10;
  const n = normalize(d);
  assert.equal(n.obs.password, null);
  assert.equal(n.timing.hold_ms, 50);
});

test('isDirty ignores equivalent forms', () => {
  const base = settingsOf(cfg());
  const d = settingsOf(cfg());
  assert.equal(isDirty(d, base), false);
  d.obs.password = '';
  assert.equal(isDirty(d, base), false);
  d.sound.master_volume = 40;
  assert.equal(isDirty(d, base), true);
});

test('addSoundFile gives unique ids', () => {
  const d = settingsOf(cfg());
  assert.equal(addSoundFile(d, 'C:/x/Clap.wav').id, 'clap-2');
  assert.equal(d.sound.sounds.length, 4);
});
