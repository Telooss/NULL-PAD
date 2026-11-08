import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createDraft, canBind } from '../js/draft.js';

const cfg = () => ({
  version: 1,
  profiles: [
    { id: 1, name: 'A', tag: 'AAA', bindings: [{ input: 'K04', trigger: 'Press', action: { type: 'Text', text: 'a' } }] },
    { id: 2, name: 'B', tag: 'BBB', bindings: [{ input: 'K00', trigger: 'Press', action: { type: 'Text', text: 'b' } }] },
  ],
  sound: { sounds: [] }, obs: { port: 1 }, midi: { port: 'x' },
});

test('merged touches only the profile, on a copy', () => {
  const c = cfg(); const orig = structuredClone(c);
  const d = createDraft(c, 2);
  d.set('K01', 'Press', { type: 'Text', text: 'n' });
  d.setProfileMeta({ name: 'B2', tag: 'BB2' });
  const m = d.merged();
  assert.deepEqual(c, orig);
  assert.deepEqual(m.profiles[0], orig.profiles[0]);
  assert.deepEqual({ s: m.sound, o: m.obs, i: m.midi }, { s: orig.sound, o: orig.obs, i: orig.midi });
  assert.equal(m.profiles[1].name, 'B2');
  assert.equal(m.profiles[1].bindings.length, 2);
});
test('set null removes; set replaces', () => {
  const d = createDraft(cfg(), 2);
  d.set('K00', 'Press', { type: 'Text', text: 'z' });
  assert.equal(d.get().bindings.length, 1);
  d.set('K00', 'Press', null);
  assert.equal(d.get().bindings.length, 0);
});
test('dirty returns to false when restored', () => {
  const d = createDraft(cfg(), 2);
  assert.equal(d.dirty(), false);
  d.set('K00', 'Press', { type: 'Text', text: 'z' });
  assert.equal(d.dirty(), true);
  d.set('K00', 'Press', { type: 'Text', text: 'b' });
  assert.equal(d.dirty(), false);
  d.setProfileMeta({ name: 'X', tag: 'BBB' });
  assert.equal(d.dirty(), true);
});
test('canBind', () => {
  const natives = { K00: '7', M1: 'P1', ENC_L_CW: 'V' };
  assert.equal(canBind(1, 'K00', natives), false);
  assert.equal(canBind(2, 'K00', natives), true);
  assert.equal(canBind(2, 'M2', natives), false);
  assert.equal(canBind(3, 'ENC_L_PRESS', natives), false);
  assert.equal(canBind(1, 'K01', natives), true);
  assert.equal(canBind(1, 'ENC_R_CW', natives), true);
});
test('addSound lands in merged sound.sounds and is dirty', () => {
  const c = cfg(); c.sound.sounds = [{ id: 'a' }]; const orig = structuredClone(c);
  const d = createDraft(c, 2);
  d.addSound({ id: 'a' });
  assert.equal(d.dirty(), false);
  d.addSound({ id: 'b', path: 'x', volume: 100, on_repeat: 'Restart' });
  assert.equal(d.dirty(), true);
  const m = d.merged();
  assert.deepEqual(m.sound.sounds.map((s) => s.id), ['a', 'b']);
  assert.deepEqual({ o: m.obs, i: m.midi, p0: m.profiles[0] }, { o: orig.obs, i: orig.midi, p0: orig.profiles[0] });
  assert.deepEqual(c, orig);
});
