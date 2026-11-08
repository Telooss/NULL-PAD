import { test } from 'node:test';
import assert from 'node:assert/strict';
import { saveFlow, guard } from '../js/config-mode.js';

// Brouillon factice : `discarded` passe a true seulement si saveFlow appelle discard.
function deps(over = {}) {
  const log = [];
  const d = {
    json: '{}', discarded: false, log,
    validate: async () => { log.push('validate'); },
    save: async () => { log.push('save'); },
    waitConfigEvent: (ms) => { log.push(`wait ${ms}`); return { promise: Promise.resolve(over.event === undefined ? { ok: true } : over.event), cancel: () => log.push('cancel') }; },
    engineUp: () => true,
    ...over,
  };
  d.discard = () => { d.discarded = true; };
  return d;
}

test('saved: config event ok, draft discarded', async () => {
  const d = deps();
  assert.deepEqual(await saveFlow(d), { state: 'saved' });
  assert.deepEqual(d.log, ['validate', 'wait 3000', 'save']);
  assert.equal(d.discarded, true);
});
test('saved-offline: engine down, no wait', async () => {
  const d = deps({ engineUp: () => false });
  assert.deepEqual(await saveFlow(d), { state: 'saved-offline' });
  assert.deepEqual(d.log, ['validate', 'save']);
  assert.equal(d.discarded, true);
});
test('saved-offline: no config event within the timeout', async () => {
  const d = deps({ event: null });
  assert.deepEqual(await saveFlow(d), { state: 'saved-offline' });
  assert.equal(d.discarded, true);
});
test('invalid: validation error, nothing written, draft kept', async () => {
  const d = deps({ validate: async () => { throw 'son "x" absent'; } });
  assert.deepEqual(await saveFlow(d), { state: 'invalid', error: 'son "x" absent' });
  assert.ok(!d.log.includes('save'));
  assert.equal(d.discarded, false);
});
test('invalid: write fails, draft kept', async () => {
  const d = deps({ save: async () => { throw new Error('disque plein'); } });
  const r = await saveFlow(d);
  assert.equal(r.state, 'invalid');
  assert.equal(r.write, true);
  assert.match(r.error, /disque plein/);
  assert.deepEqual(d.log, ['validate', 'wait 3000', 'cancel']);
  assert.equal(d.discarded, false);
});
test('rejected: engine refuses, draft kept', async () => {
  const d = deps({ event: { ok: false, error: 'version' } });
  assert.deepEqual(await saveFlow(d), { state: 'rejected', error: 'version' });
  assert.equal(d.discarded, false);
});
test('guard', () => {
  assert.equal(guard(false, 'switch-profile'), 'proceed');
  assert.equal(guard(true, 'switch-profile'), 'ask');
  assert.equal(guard(true, 'close'), 'ask');
  assert.equal(guard(true, 'cancel'), 'ask');
  assert.equal(guard(true, 'save'), 'proceed');
});
