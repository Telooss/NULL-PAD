import { test } from 'node:test';
import assert from 'node:assert/strict';
import { suggestTag, cleanTag, nextFreeId, withNewProfile, withoutProfile, barSlots } from '../js/profiles.js';

const mk = (...ids) => ({ version: 1, profiles: ids.map((id) => ({ id, name: `P${id}`, tag: `T${id}X`, bindings: [{ input: 'K00', trigger: 'Press', action: { type: 'MicMute' } }] })) });

test('suggestTag: initiales puis remplissage, accents retires, X en bourrage', () => {
  assert.equal(suggestTag('STREAM NIGHT'), 'STN');
  assert.equal(suggestTag('stream'), 'STR');
  assert.equal(suggestTag('ab'), 'ABX');
  assert.equal(suggestTag(''), 'XXX');
  assert.equal(suggestTag('Été'), 'ETE');
  assert.equal(suggestTag('a-b_c d'), 'ABC');
  assert.equal(suggestTag('!!'), 'XXX');
});

test('cleanTag: A-Z0-9 majuscules, 3 max', () => assert.equal(cleanTag('a-b1cd'), 'AB1'));

test('nextFreeId: plus petit libre, null si plein', () => {
  assert.equal(nextFreeId(mk(1, 2)), 3);
  assert.equal(nextFreeId(mk(1, 3)), 2);
  assert.equal(nextFreeId(mk(1, 2, 3, 4, 5, 6, 7, 8)), null);
  assert.equal(nextFreeId({ profiles: [] }), 1);
});

test('withNewProfile: vide ou copie, ne modifie pas l original, tri par id', () => {
  const c = mk(1, 3);
  const a = withNewProfile(c, { name: 'Neuf', tag: 'NEU' });
  assert.deepEqual(a.profiles.map((p) => p.id), [1, 2, 3]);
  assert.deepEqual(a.profiles[1], { id: 2, name: 'Neuf', tag: 'NEU', bindings: [] });
  const b = withNewProfile(c, { name: 'Copie', tag: 'CPY', copyFrom: 3 });
  assert.deepEqual(b.profiles[1].bindings, c.profiles[1].bindings);
  assert.notEqual(b.profiles[1].bindings, c.profiles[1].bindings);
  assert.equal(c.profiles.length, 2);
  assert.throws(() => withNewProfile(mk(1, 2, 3, 4, 5, 6, 7, 8), { name: 'x', tag: 'XXX' }));
});

test('withoutProfile: retire un profil, jamais le 1', () => {
  assert.deepEqual(withoutProfile(mk(1, 2, 3), 2).profiles.map((p) => p.id), [1, 3]);
  assert.throws(() => withoutProfile(mk(1, 2), 1));
});

test('barSlots: emplacements M1-M3 (null si vide) puis profils 4-8', () => {
  assert.deepEqual(barSlots(mk(1, 3, 5)).map((s) => s && s.id), [1, null, 3, 5]);
  assert.deepEqual(barSlots(null), [null, null, null]);
});
