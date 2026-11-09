import { test } from 'node:test';
import assert from 'node:assert/strict';
import { shouldCheck, describe, progressText } from '../js/update.js';

const H = 3600 * 1000;
const now = Date.UTC(2026, 10, 9, 12);

test('shouldCheck : au demarrage, puis toutes les 6 h', () => {
  assert.equal(shouldCheck(null, now, true), true);
  assert.equal(shouldCheck(now - (6 * H - 60 * 1000), now, true), false); // 5 h 59
  assert.equal(shouldCheck(now - 6 * H, now, true), true);
  assert.equal(shouldCheck(now - 30 * H, now, true), true);
});

test('shouldCheck : jamais quand le reglage est coupe', () => {
  assert.equal(shouldCheck(null, now, false), false);
  assert.equal(shouldCheck(now - 30 * H, now, false), false);
});

test('shouldCheck : date illisible ou dans le futur (horloge reculee) -> verifier', () => {
  assert.equal(shouldCheck(NaN, now, true), true);
  assert.equal(shouldCheck(now + H, now, true), true);
});

test('describe : un message court par code', () => {
  const codes = ['network', 'config', 'signature', 'engine', 'install'];
  const msgs = codes.map((c) => describe(`${c}: detail technique`));
  assert.equal(new Set(msgs).size, codes.length);
  for (const m of msgs) assert.ok(!m.includes('detail technique') && m.length < 80, m);
  assert.match(describe('network: dns'), /connexion/i);
  assert.match(describe('signature: bad'), /signature/i);
  assert.match(describe('engine: le moteur ne s’est pas arrêté'), /moteur/i);
});

test('describe : erreur inconnue -> message generique avec le detail', () => {
  assert.equal(describe('boom'), 'Mise à jour impossible : boom');
  assert.equal(describe(new Error('x')), 'Mise à jour impossible : x');
});

test('progressText : pourcentage, puis installation ; Mo sans taille connue', () => {
  assert.deepEqual(progressText(0, 200), { pct: 0, text: '0 %' });
  assert.deepEqual(progressText(50, 200), { pct: 25, text: '25 %' });
  assert.deepEqual(progressText(200, 200), { pct: 100, text: 'Installation…' });
  assert.deepEqual(progressText(3_500_000, null), { pct: null, text: '3.5 Mo' });
});
