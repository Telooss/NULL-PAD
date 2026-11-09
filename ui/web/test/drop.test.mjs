import { test } from 'node:test';
import assert from 'node:assert/strict';
import { actionForDrop } from '../js/drop.js';

const natives = { K00: '7' };
const d = (paths, target = 'K05', pid = 1, ids = []) => actionForDrop(paths, target, pid, natives, ids);

test('audio -> Sound with unique id', () => {
  const r = d(['C:\\Sons\\Air Horn!.WAV'], 'K05', 1, ['air-horn']);
  assert.deepEqual(r.action, { type: 'Sound', id: 'air-horn-2' });
  assert.deepEqual(r.sound, { id: 'air-horn-2', path: 'C:\\Sons\\Air Horn!.WAV', volume: 100, on_repeat: 'Restart' });
  assert.equal(d(['a.mp3'], 'K05', 1, ['a', 'a-2']).action.id, 'a-3');
  for (const e of ['mp3', 'ogg', 'flac']) assert.equal(d([`x.${e}`]).action.type, 'Sound');
});
test('programs and scripts', () => {
  assert.deepEqual(d(['C:\\a.exe']).action, { type: 'Launch', program: 'C:\\a.exe', args: [] });
  assert.deepEqual(d(['C:\\a.lnk']).action, { type: 'Open', target: 'C:\\a.lnk' });
  assert.deepEqual(d(['C:\\a.pdf']).action, { type: 'Open', target: 'C:\\a.pdf' });
});
test('scripts are structured Launch actions, paths untouched', () => {
  for (const p of ["C:\\it's $x & y\\a.bat", 'C:\\L\u2019air \u2018x\u2019.CMD', 'C:\\a b.bat']) {
    assert.deepEqual(d([p]).action, { type: 'Launch', program: p, args: [] });
  }
  for (const p of ["C:\\it's $x & y\\a.ps1", 'C:\\L\u2019air.ps1', 'C:\\a b.ps1']) {
    assert.deepEqual(d([p]).action, { type: 'Launch', program: 'powershell.exe', args: ['-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', p] });
  }
});
test('several files', () => {
  const r = d(['a.pdf', 'b.pdf']);
  assert.equal(r.action.target, 'a.pdf');
  assert.equal(r.warning, 'Seul le premier fichier est utilisé');
});
test('refused targets', () => {
  assert.equal(d(['a.pdf'], 'K00', 1).error, 'Touche native dans le pad : branchez le pad en USB pour la modifier');
  assert.equal(d(['a.pdf'], 'K00', 2).action.type, 'Open');
  assert.equal(d(['a.pdf'], 'M2', 2).error, 'Touche de profil du firmware : non modifiable ici');
  assert.equal(d(['a.pdf'], 'ENC_L_CW', 2).error, 'Molette gauche du firmware : non modifiable ici');
});
