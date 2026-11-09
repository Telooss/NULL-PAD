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

// ── Pad d'abord, config ensuite ──
const PAD_A = { layers: [{ id: 0, name: 'DEF', bindings: [] }] };
const PAD_B = { layers: [{ id: 0, name: 'NEW', bindings: [] }] };
function padDeps(over = {}, padOver = {}) {
  const d = deps(over);
  d.validate = async (json, pad) => { d.log.push(`validate ${pad?.layers[0].name ?? null}`); };
  d.save = async (json, pad) => { d.log.push(`save ${pad?.layers[0].name ?? null}`); if (over.saveFails) throw 'disque plein'; };
  d.pad = {
    expected: PAD_A, target: PAD_B,
    write: async (e, t) => { d.log.push(`pad_write ${e.layers[0].name}>${t.layers[0].name}`); return structuredClone(t); },
    onForeign: async () => { d.log.push('foreign?'); return true; },
    ...padOver,
  };
  return d;
}

test('pad first, then config with the returned model', async () => {
  const d = padDeps();
  const r = await saveFlow(d);
  assert.equal(r.state, 'saved');
  assert.equal(r.padUpdated, true);
  assert.deepEqual(r.padModel, PAD_B);
  assert.deepEqual(d.log, ['validate NEW', 'pad_write DEF>NEW', 'wait 3000', 'save NEW']);
  assert.equal(d.discarded, true);
});
test('unchanged pad: no pad_write, config validated and saved with that pad', async () => {
  const d = padDeps({}, { target: structuredClone(PAD_A) });
  const r = await saveFlow(d);
  assert.deepEqual(r, { state: 'saved' });
  assert.deepEqual(d.log, ['validate DEF', 'wait 3000', 'save DEF']);
});
for (const code of ['locked', 'cancelled', 'timeout', 'changed-elsewhere', 'disconnected', 'absent', 'guard', 'rejected', 'busy']) {
  test(`pad error ${code}: config not written, draft kept`, async () => {
    const d = padDeps({}, { write: async () => { d.log.push('pad_write'); throw `${code}: message`; } });
    const r = await saveFlow(d);
    assert.deepEqual(r, { state: 'pad', code, error: 'message' });
    assert.ok(!d.log.some((l) => l.startsWith('save')));
    assert.equal(d.discarded, false);
  });
}
test('foreign-unsaved: confirmed, discarded, write retried once', async () => {
  let n = 0;
  const d = padDeps({}, { write: async (e, t) => { d.log.push('pad_write'); if (!n++) throw 'foreign-unsaved: ailleurs'; return t; } });
  const r = await saveFlow(d);
  assert.equal(r.state, 'saved');
  assert.deepEqual(d.log, ['validate NEW', 'pad_write', 'foreign?', 'pad_write', 'wait 3000', 'save NEW']);
});
test('foreign-unsaved twice or declined: pad error, nothing written', async () => {
  const d = padDeps({}, { write: async () => { d.log.push('pad_write'); throw 'foreign-unsaved: ailleurs'; } });
  assert.equal((await saveFlow(d)).code, 'foreign-unsaved');
  assert.deepEqual(d.log, ['validate NEW', 'pad_write', 'foreign?', 'pad_write']);
  const d2 = padDeps({}, { write: async () => { throw 'foreign-unsaved: ailleurs'; }, onForeign: async () => false });
  assert.deepEqual(await saveFlow(d2), { state: 'pad', code: 'foreign-unsaved', error: 'ailleurs' });
});
test('invalid config: pad never written', async () => {
  const d = padDeps();
  d.validate = async () => { d.log.push('validate'); throw 'touche native'; };
  assert.equal((await saveFlow(d)).state, 'invalid');
  assert.deepEqual(d.log, ['validate']);
});
test('config fails after pad success: padUpdated, draft kept', async () => {
  const d = padDeps({ saveFails: true });
  const r = await saveFlow(d);
  assert.equal(r.state, 'invalid');
  assert.equal(r.write, true);
  assert.equal(r.padUpdated, true);
  assert.deepEqual(r.padModel, PAD_B);
  assert.equal(d.discarded, false);
});
