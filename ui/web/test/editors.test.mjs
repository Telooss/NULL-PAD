import { test } from 'node:test';
import assert from 'node:assert/strict';
import { fieldsOf, applyField, defaultAction, moveStep, addStep, removeStep, setStep, shortcutOf, NESTED_TYPES } from '../js/editors/model.js';

const field = (a, key, sounds) => fieldsOf(a, sounds).find((f) => f.key === key);

test('Shortcut: keys field and capture', () => {
  const a = { type: 'Shortcut', keys: 'Ctrl+C' };
  assert.equal(field(a, 'keys').value, 'Ctrl+C');
  assert.deepEqual(applyField(a, 'keys', 'Alt+F4').action, { type: 'Shortcut', keys: 'Alt+F4' });
  assert.equal(shortcutOf({ code: 'KeyK', ctrlKey: true, shiftKey: true }), 'Ctrl+Shift+K');
  assert.equal(shortcutOf({ code: 'ArrowUp', altKey: true }), 'Alt+Up');
  assert.equal(shortcutOf({ code: 'F13' }), 'F13');
  assert.equal(shortcutOf({ code: 'ControlLeft', ctrlKey: true }), null);
});
test('Launch: args one per line <-> array; URL check switches to Open', () => {
  const a = { type: 'Launch', program: 'C:\a.exe', args: ['-x', 'y z'] };
  assert.equal(field(a, 'args').value, '-x\ny z');
  assert.deepEqual(applyField(a, 'args', '-a\r\n\n  \n-b c\n').action.args, ['-a', '-b c']);
  const o = applyField(a, 'open', true).action;
  assert.deepEqual(o, { type: 'Open', target: 'C:\a.exe' });
  assert.equal(field(o, 'open').value, true);
  assert.deepEqual(applyField(o, 'open', false).action, { type: 'Launch', program: 'C:\a.exe', args: [] });
});
test('Macro steps: add, move, remove, set', () => {
  let s = addStep([], 'Do');
  s = addStep(s, 'Wait');
  assert.deepEqual(s.map((x) => x.type), ['Do', 'Wait']);
  assert.deepEqual(moveStep(s, 1, -1).map((x) => x.type), ['Wait', 'Do']);
  assert.equal(moveStep(s, 1, 1), s);
  assert.deepEqual(removeStep(s, 0), [{ type: 'Wait', ms: 100 }]);
  s = setStep(s, 1, { type: 'Wait', ms: 250 });
  const m = applyField({ type: 'Macro', steps: [] }, 'steps', s).action;
  assert.deepEqual(m.steps[1], { type: 'Wait', ms: 250 });
  assert.equal(field(m, 'steps').value.length, 2);
});
test('Toggle: on/off sub-editors, nested Toggle refused', () => {
  const t = defaultAction('Toggle');
  assert.deepEqual(fieldsOf(t).map((f) => [f.key, f.kind]), [['on', 'sub'], ['off', 'sub']]);
  const t2 = applyField(t, 'on', { type: 'Media', key: 'Next' }).action;
  assert.deepEqual(t2.on, { type: 'Media', key: 'Next' });
  assert.equal(applyField(t2, 'off', defaultAction('Toggle')).action, t2);
  assert.ok(!NESTED_TYPES.includes('Toggle'));
});
test('Sound: volume and on_repeat go to the SoundDef', () => {
  const sounds = [{ id: 'clap', path: 'c.wav', volume: 80, on_repeat: 'Stop' }];
  const a = { type: 'Sound', id: 'clap' };
  assert.equal(field(a, 'volume', sounds).value, 80);
  assert.equal(field(a, 'on_repeat', sounds).value, 'Stop');
  assert.deepEqual(applyField(a, 'volume', '140', sounds), { action: a, soundPatch: { id: 'clap', patch: { volume: 100 } } });
  assert.deepEqual(applyField(a, 'on_repeat', 'Restart', sounds).soundPatch, { id: 'clap', patch: { on_repeat: 'Restart' } });
  assert.equal(field({ type: 'Sound', id: 'nope' }, 'volume', sounds), undefined);
  assert.deepEqual(defaultAction('Sound', sounds), { type: 'Sound', id: 'clap' });
});
test('MIDI and volume bounds', () => {
  const n = { type: 'MidiNote', channel: 1, note: 60, velocity: 100 };
  assert.equal(applyField(n, 'channel', 0).action.channel, 1);
  assert.equal(applyField(n, 'channel', 17).action.channel, 16);
  assert.equal(applyField(n, 'note', 200).action.note, 127);
  assert.equal(applyField(n, 'velocity', 0).action.velocity, 1);
  assert.equal(applyField(n, 'note', 'abc').action.note, 60);
  const c = { type: 'MidiCc', channel: 2, cc: 7, value: 64 };
  assert.equal(applyField(c, 'value', -5).action.value, 0);
  assert.equal(applyField({ type: 'Volume', delta: 2 }, 'delta', -300).action.delta, -100);
  assert.deepEqual(applyField(n, 'type', 'MidiCc').action, defaultAction('MidiCc'));
});
