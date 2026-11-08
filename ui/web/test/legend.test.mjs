import { test } from 'node:test';
import assert from 'node:assert/strict';
import { legendFor, fitLegend } from '../js/legend.js';
import { ACTIONS, TRIGGERS } from '../js/catalog.js';

const natives = { K00: '7', K11: '+', M1: 'P1', ENC_L_CW: 'VOL+' };
const b = (action, label) => ({ input: 'K05', trigger: 'Press', action, ...(label ? { label } : {}) });

test('label wins', () => {
  assert.deepEqual(legendFor(b({ type: 'Sound', id: 'x' }, 'Copier'), natives, []), { text: 'Copier', kind: 'action' });
});
test('action legends', () => {
  const t = (a) => legendFor(b(a), natives, []).text;
  assert.equal(t({ type: 'Sound', id: 'airhorn' }), 'AIRHORN');
  assert.equal(t({ type: 'ObsScene', scene: 'Jeu' }), 'Jeu');
  assert.equal(t({ type: 'Shortcut', keys: 'Ctrl+c' }), 'CTRL+C');
  assert.equal(t({ type: 'Volume', delta: 2 }), 'VOL+');
  assert.equal(t({ type: 'Volume', delta: -2 }), 'VOL−');
  assert.equal(t({ type: 'Launch', program: 'C:\\Apps\\Foo.exe', args: [] }), 'Foo');
  assert.equal(t({ type: 'MidiNote', channel: 1, note: 60, velocity: 100 }), 'N60');
  assert.equal(t({ type: 'MidiCc', channel: 1, cc: 20, value: 1 }), 'CC20');
});
test('native, profile, free', () => {
  assert.deepEqual(legendFor(null, natives, [], 'K00'), { text: '7', kind: 'native' });
  assert.deepEqual(legendFor(null, natives, [], 'M1'), { text: 'P1', kind: 'profile' });
  assert.deepEqual(legendFor(null, natives, [], 'K07'), { text: '+', kind: 'free' });
});
test('fitLegend', () => {
  assert.deepEqual(fitLegend('CTRL+C'), ['CTRL+C']);
  assert.deepEqual(fitLegend('SCENE JEU PRINCIPALE'), ['SCENE', 'JEU P…']);
  assert.deepEqual(fitLegend('CTRL+SHIFT'), ['CTRL+', 'SHIFT']);
  assert.deepEqual(fitLegend('ABCDEFGHIJKLMNOP'), ['ABCDEF', 'GHIJK…']);
});
test('catalog', () => {
  assert.equal(ACTIONS.length, 11);
  assert.ok(ACTIONS.every((a) => a.type && a.label && a.desc && a.icon));
  assert.deepEqual(TRIGGERS, ['Press', 'Hold', 'DoubleTap']);
});
test('more fallbacks', () => {
  const t = (a) => legendFor(b(a), {}, []).text;
  assert.equal(t({ type: 'Toggle', on: { type: 'Sound', id: 'x' }, off: { type: 'StopSounds' } }), 'X');
  assert.equal(t({ type: 'Open', target: 'C:\\docs\\' }), 'docs');
  assert.equal(t({ type: 'Open', target: 'https://a.b/' }), 'a.b');
  assert.equal(t({ type: 'Launch', program: 'C:\\Apps\\' }), 'Apps');
  assert.equal(t({ type: 'Command', line: 'x' }), 'CMD');
  assert.equal(t({ type: 'Text', text: 'bonjour' }), 'BONJOUR');
  assert.equal(t({ type: 'Media', key: 'PlayPause' }), 'PLAY');
  assert.equal(t({ type: 'MicVolume', delta: -1 }), 'MIC−');
  assert.equal(t({ type: 'SoundVolume', delta: 1 }), 'SON+');
  assert.equal(t({ type: 'SoundVolume', delta: -1 }), 'SON−');
});
