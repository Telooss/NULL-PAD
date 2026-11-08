import { test } from 'node:test';
import assert from 'node:assert/strict';
import { inputOf, pieceOf } from '../js/pad.js';

test('pieces -> inputs', () => {
  assert.equal(inputOf('F1'), 'K00');
  assert.equal(inputOf('F20'), 'K19');
  assert.equal(inputOf('F21'), 'M1');
  assert.equal(inputOf('F23'), 'M3');
  assert.equal(inputOf('D1'), 'ENC_L_PRESS');
  assert.equal(inputOf('D2'), 'ENC_R_PRESS');
  assert.equal(inputOf('PCB'), null);
  assert.equal(inputOf(null), null);
});
test('inputs -> pieces, round trip', () => {
  for (let i = 1; i <= 23; i++) assert.equal(pieceOf(inputOf(`F${i}`)), `F${i}`);
  assert.equal(pieceOf('ENC_L_CW'), 'D1');
  assert.equal(pieceOf('ENC_R_CCW'), 'D2');
  assert.equal(pieceOf('ENC_R_PRESS'), 'D2');
  assert.equal(pieceOf('K20'), null);
  assert.equal(pieceOf('X'), null);
});
