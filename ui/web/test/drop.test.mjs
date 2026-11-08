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
  assert.deepEqual(d(['C:\\a.ps1']).action, { type: 'Command', line: "& powershell -NoProfile -ExecutionPolicy Bypass -File 'C:\\a.ps1'" });
  assert.deepEqual(d(['C:\\a.bat']).action, { type: 'Command', line: "& 'C:\\a.bat'" });
  assert.equal(d(['C:\\a.CMD']).action.type, 'Command');
  assert.equal(d(["C:\\it's $x\\a.bat"]).action.line, "& 'C:\\it''s $x\\a.bat'");
  assert.deepEqual(d(['C:\\a.pdf']).action, { type: 'Open', target: 'C:\\a.pdf' });
});
test('several files', () => {
  const r = d(['a.pdf', 'b.pdf']);
  assert.equal(r.action.target, 'a.pdf');
  assert.equal(r.warning, 'Seul le premier fichier est utilisé');
});
test('refused targets', () => {
  assert.equal(d(['a.pdf'], 'K00', 1).error, 'Touche native du firmware : modifiable plus tard depuis le pad');
  assert.equal(d(['a.pdf'], 'K00', 2).action.type, 'Open');
  assert.equal(d(['a.pdf'], 'M2', 2).error, 'Touche de profil du firmware : non modifiable ici');
  assert.equal(d(['a.pdf'], 'ENC_L_CW', 2).error, 'Molette gauche du firmware : non modifiable ici');
});
