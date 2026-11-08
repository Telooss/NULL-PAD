import { test } from 'node:test';
import assert from 'node:assert/strict';
import { describeAction, legendsFor, nativesFor, panelModel, profileSlots, toastText, linkText, placeOf } from '../js/view.js';

const natives = { K04: '7', M1: 'P1', ENC_L_CW: 'VOL+' };
const cfg = { sound: { sounds: [] }, profiles: [
  { id: 1, name: 'Default', tag: 'DEF', bindings: [{ input: 'K11', trigger: 'Press', action: { type: 'Shortcut', keys: 'Ctrl+C' } }] },
  { id: 2, name: 'Stream', tag: 'STR', bindings: [
    { input: 'K04', trigger: 'Hold', action: { type: 'Toggle', on: { type: 'ObsRecord', op: 'Toggle' }, off: { type: 'Sound', id: 'a' } } },
    { input: 'ENC_R_PRESS', trigger: 'Press', action: { type: 'MicMute' } }] },
] };

test('describeAction', () => {
  assert.deepEqual(describeAction({ type: 'Sound', id: 'airhorn' }), { title: 'Son', desc: 'airhorn', icon: 'volume-2' });
  assert.equal(describeAction({ type: 'ObsScene', scene: 'Jeu' }).title, 'OBS');
  assert.equal(describeAction({ type: 'MicMute' }).title, 'Volume et micro');
  assert.equal(describeAction({ type: 'Toggle', on: { type: 'ObsRecord', op: 'Toggle' }, off: { type: 'Sound', id: 'a' } }).desc, 'OBS ⇄ Son');
});
test('natives only on profile 1', () => {
  assert.deepEqual(Object.keys(nativesFor(natives, 2)), ['M1', 'ENC_L_CW']);
  assert.equal(legendsFor(cfg, 1, natives).K04.text[0], '7');
  assert.equal(legendsFor(cfg, 2, natives).K04.kind, 'free');
  assert.equal(legendsFor(cfg, 2, natives).K05.kind, 'free');
  assert.equal(legendsFor(cfg, 1, natives).K11.text[0], 'CTRL+C');
});
test('panelModel', () => {
  assert.equal(panelModel(cfg, 1, 'K04', natives).kind, 'native');
  assert.match(panelModel(cfg, 1, 'K04', natives).note, /^Touche native · 7, envoyée par le pad/);
  assert.equal(panelModel(cfg, 1, 'M1', natives).note, 'Touche de profil du firmware');
  const k = panelModel(cfg, 2, 'K04', natives);
  assert.equal(k.sections[0].rows.length, 3);
  assert.equal(k.sections[0].rows[0].action, null);
  assert.equal(k.sections[0].rows[1].action.title, 'Bascule');
  const e = panelModel(cfg, 2, 'ENC_R_PRESS', natives);
  assert.equal(e.sections.length, 3);
  assert.equal(e.sections[2].rows[0].action.title, 'Volume et micro');
  assert.ok(panelModel(cfg, 2, 'ENC_L_CW', natives).sections[0].native);
});
test('slots, toasts, link', () => {
  assert.deepEqual(profileSlots(cfg).map((s) => s && s.tag), ['DEF', 'STR', null]);
  assert.equal(toastText({ input: 'K07', ok: true, kind: 'Sound' }, { action: { type: 'Sound', id: 'airhorn' } }), 'K07 → Son : airhorn');
  assert.equal(toastText({ input: 'K07', ok: true, kind: 'Sound' }), 'K07 → Sound');
  assert.equal(toastText({ input: 'K07', ok: false, error: 'OBS injoignable' }), 'K07 → erreur : OBS injoignable');
  assert.equal(linkText('none'), 'Pad absent');
  assert.equal(linkText('ble'), 'BLE');
  assert.equal(placeOf('K05'), 'Rangée 2 · colonne 2');
});
