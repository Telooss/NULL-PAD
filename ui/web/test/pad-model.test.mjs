import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
  INPUTS, isAction, padLegend, padNatives, canBind, setNative, setAction, renameLayer, withLayer, copyLayer,
  parseErr, samePad, pillState, nativeOf, usageOfCode, groupCatalog, changedProfiles, retryBusy,
} from '../js/pad-model.js';
import { createDraft } from '../js/draft.js';
import { legendsFor, panelModel } from '../js/view.js';

const act = (profile, input) => ({ kind: 'action', profile, input });
const nat = (usage, label) => ({ kind: 'native', usage, label });
const layer = (p) => INPUTS.map((_, i) => act(p, i));
const model = () => {
  const m = { layers: Array.from({ length: 8 }, (_, i) => ({ id: i, name: i ? `P0${i + 1}` : 'DEF', bindings: layer(i + 1) })) };
  m.layers[0].bindings[4] = nat(0x7005f, '7');
  m.layers[0].bindings[17] = { kind: 'other', behavior: 'np_00', p1: 0, p2: 0 };
  m.layers[0].bindings[20] = { kind: 'profile_macro', n: 1 };
  m.layers[1].bindings[3] = { kind: 'other', behavior: 'bt', p1: 1, p2: 0 };
  return m;
};
const cfg = { sound: { sounds: [] }, profiles: [
  { id: 1, name: 'A', tag: 'AAA', bindings: [{ input: 'K11', trigger: 'Press', action: { type: 'Shortcut', keys: 'Ctrl+C' } }] },
  { id: 2, name: 'B', tag: 'BBB', bindings: [] },
] };
const firmware = { K04: '7', M1: 'P1', ENC_L_CW: 'VOL+' };

test('padLegend: native label, P{n}, 00, behavior name, action -> null', () => {
  assert.equal(padLegend(nat(1, 'Volume +')), 'VOLUME +');
  assert.equal(padLegend({ kind: 'profile_macro', n: 3 }), 'P3');
  assert.equal(padLegend({ kind: 'other', behavior: 'np_00', p1: 0, p2: 0 }), '00');
  assert.equal(padLegend({ kind: 'other', behavior: 'bt', p1: 0, p2: 0 }), 'bt');
  assert.equal(padLegend(act(1, 4)), null);
});

test('isAction and padNatives per profile', () => {
  const m = model();
  assert.equal(isAction(m, 1, 'K04'), false);
  assert.equal(isAction(m, 1, 'K05'), true);
  assert.equal(isAction(m, 2, 'K04'), true);
  assert.deepEqual(padNatives(m, 1), { K04: '7', K17: '00', M1: 'P1' });
  assert.deepEqual(padNatives(m, 2), { K03: 'bt' });
  m.layers[2].bindings[0] = act(2, 0); // action d'une autre place : native pour nullctl
  assert.deepEqual(padNatives(m, 3), { K00: 'ACT' });
});

test('legends from the pad model, today legends without', () => {
  const m = model();
  const l = legendsFor(cfg, 1, firmware, m);
  assert.deepEqual(l.K04, { text: ['7'], kind: 'native' });
  assert.equal(l.K17.text[0], '00');
  assert.equal(l.M1.text[0], 'P1');
  assert.equal(l.K11.text[0], 'CTRL+C'); // Action : legende nullctl
  assert.equal(l.ENC_L_CW.kind, 'free'); // Action dans ce pad
  assert.equal(legendsFor(cfg, 2, firmware, m).K03.text[0], 'bt');
  assert.equal(legendsFor(cfg, 1, firmware).ENC_L_CW.text[0], 'VOL+');
  assert.equal(panelModel(cfg, 1, 'K04', firmware, m).kind, 'native');
  assert.equal(panelModel(cfg, 1, 'M2', firmware, m).kind, 'key'); // M2 = Action dans ce pad
});

test('canBind: pad editable, read-only model, no pad', () => {
  const m = model();
  for (const i of ['K04', 'M1', 'ENC_L_CW', 'K17']) assert.equal(canBind(1, i, firmware, { model: m, editable: true }), true);
  assert.equal(canBind(1, 'K04', firmware, { model: m, editable: false }), false);
  assert.equal(canBind(1, 'K05', firmware, { model: m, editable: false }), true);
  assert.equal(canBind(1, 'ENC_L_CW', firmware, { model: m, editable: false }), true);
  assert.equal(canBind(1, 'K04', firmware), false);
  assert.equal(canBind(1, 'ENC_L_CW', firmware, null), false);
  assert.equal(canBind(2, 'K04', firmware, null), true);
});

test('native <-> action toggles: target copied, draft bindings of that input removed', () => {
  const m = model();
  const d = createDraft(structuredClone(cfg), 1);
  d.set('K05', 'Press', { type: 'MicMute' });
  d.set('K05', 'Hold', { type: 'Text', text: 'x' });
  d.set('K06', 'Press', { type: 'MicMute' });
  const t = setNative(m, d, 1, 'K05', nat(0x70060, '8'));
  assert.deepEqual(t.layers[0].bindings[5], nat(0x70060, '8'));
  assert.deepEqual(m.layers[0].bindings[5], act(1, 5)); // original intact
  assert.deepEqual(d.get().bindings.map((b) => b.input).sort(), ['K06', 'K11']);
  const back = setAction(t, 1, 'K05');
  assert.deepEqual(back.layers[0].bindings[5], act(1, 5));
  assert.ok(samePad(back, m));
  assert.ok(!samePad(t, m));
});

test('renameLayer: tag -> layer name, cut to max_name_len', () => {
  const m = model();
  assert.equal(renameLayer(m, 2, 'STR', 16).layers[1].name, 'STR');
  assert.equal(renameLayer(m, 2, 'STR', 2).layers[1].name, 'ST');
  assert.equal(m.layers[1].name, 'P02');
});

test('copyLayer remaps Action{src} to Action{dst}; withLayer sets one layer', () => {
  const src = [act(2, 0), nat(5, 'B'), { kind: 'profile_macro', n: 1 }, act(1, 3)];
  assert.deepEqual(copyLayer(src, 2, 5), [act(5, 0), nat(5, 'B'), { kind: 'profile_macro', n: 1 }, act(1, 3)]);
  const m = model();
  const w = withLayer(m, 5, layer(9), 'NEW');
  assert.equal(w.layers[4].name, 'NEW');
  assert.deepEqual(w.layers[4].bindings[0], act(9, 0));
  assert.equal(m.layers[4].name, 'P05');
});

test('parseErr', () => {
  assert.deepEqual(parseErr('changed-elsewhere: Le pad a changé'), { code: 'changed-elsewhere', message: 'Le pad a changé' });
  assert.deepEqual(parseErr(new Error('locked: Pad verrouillé')), { code: 'locked', message: 'Pad verrouillé' });
  assert.deepEqual(parseErr('boom'), { code: null, message: 'boom' });
});

test('pillState', () => {
  assert.equal(pillState({ present: true, locked: false, busy: false }, false), 'editable');
  assert.equal(pillState({ present: true, locked: true, busy: false }, false), 'locked');
  assert.equal(pillState({ present: false, locked: null, busy: false }, false), 'absent');
  assert.equal(pillState({ present: true, locked: null, busy: true }, false), 'busy');
  assert.equal(pillState(null, false), 'absent');
  assert.equal(pillState({ present: true, locked: false, busy: false }, true), 'stale');
});

test('nativeOf composes ZMK implicit modifiers; usageOfCode maps KeyboardEvent.code', () => {
  const c = { usage: 0x70006, label: 'C', group: 'Clavier' };
  assert.deepEqual(nativeOf(c, []), { kind: 'native', usage: 0x70006, label: 'C' });
  assert.deepEqual(nativeOf(c, ['ctrl']), { kind: 'native', usage: 0x01070006, label: 'Ctrl+C' });
  assert.deepEqual(nativeOf(c, ['win', 'shift', 'ctrl', 'alt']), { kind: 'native', usage: 0x0f070006, label: 'Ctrl+Maj+Alt+Win+C' });
  assert.equal(usageOfCode('KeyC'), 0x70006);
  assert.equal(usageOfCode('Digit0'), 0x70027);
  assert.equal(usageOfCode('Digit1'), 0x7001e);
  assert.equal(usageOfCode('Numpad7'), 0x7005f);
  assert.equal(usageOfCode('Numpad0'), 0x70062);
  assert.equal(usageOfCode('F13'), 0x70068);
  assert.equal(usageOfCode('NumpadEnter'), 0x70058);
  assert.equal(usageOfCode('ArrowUp'), 0x70052);
  assert.equal(usageOfCode('ControlLeft'), null); // modificateur seul : on attend la touche
  assert.equal(usageOfCode('IntlBackslash'), null);
});

test('groupCatalog filters by label or group and keeps group order', () => {
  const cat = [{ usage: 1, label: '1', group: 'Clavier' }, { usage: 2, label: 'Entrée', group: 'Clavier' },
    { usage: 3, label: '1', group: 'Pavé numérique' }, { usage: 4, label: 'Muet', group: 'Média et volume' }];
  assert.deepEqual(groupCatalog(cat, '').map(([g, ks]) => [g, ks.length]), [['Clavier', 2], ['Pavé numérique', 1], ['Média et volume', 1]]);
  assert.deepEqual(groupCatalog(cat, '1').map(([g]) => g), ['Clavier', 'Pavé numérique']);
  assert.deepEqual(groupCatalog(cat, 'entree').map(([, ks]) => ks[0].usage), [2]);
  assert.deepEqual(groupCatalog(cat, 'pavé').map(([g]) => g), ['Pavé numérique']);
});

test('changedProfiles lists the written layers', () => {
  const a = model();
  assert.deepEqual(changedProfiles(a, structuredClone(a)), []);
  const b = renameLayer(setAction(a, 1, 'K04'), 3, 'JEU', 16);
  assert.deepEqual(changedProfiles(a, b), [1, 3]);
});

test('retryBusy: one retry on busy only', async () => {
  let n = 0;
  const slept = [];
  const sleep = async () => { slept.push(1); };
  assert.equal(await retryBusy(async () => (n++ ? 'ok' : Promise.reject('busy: Pad occupé')), sleep), 'ok');
  assert.equal(slept.length, 1);
  await assert.rejects(retryBusy(async () => { throw 'busy: Pad occupé'; }, sleep), /busy/);
  await assert.rejects(retryBusy(async () => { throw 'locked: x'; }, sleep), /locked/);
  assert.equal(slept.length, 2);
});
