// Fichier depose sur une touche -> action. Pur : aucun acces DOM.
import { canBind } from './draft.js';

// Le moteur lance `powershell -NoProfile -Command <line>` : chemin entre apostrophes (apostrophe doublee), pas d'expansion de $.
const q = (p) => `'${p.replace(/'/g, "''")}'`;
const AUDIO = ['mp3', 'wav', 'ogg', 'flac'];

function refusal(target, profileId, natives) {
  if (canBind(profileId, target, natives)) return null;
  if (/^M[123]$/.test(target)) return 'Touche de profil du firmware : non modifiable ici';
  if (target.startsWith('ENC_L_')) return 'Molette gauche du firmware : non modifiable ici';
  return 'Touche native du firmware : modifiable plus tard depuis le pad';
}

export function actionForDrop(paths, target, profileId, natives = {}, existingSoundIds = []) {
  const error = refusal(target, profileId, natives);
  if (error) return { error };
  if (!paths?.length) return { error: 'Aucun fichier' };
  const p = paths[0];
  const out = {};
  if (paths.length > 1) out.warning = 'Seul le premier fichier est utilisé';
  const name = p.split(/[\\/]/).pop();
  const ext = name.includes('.') ? name.split('.').pop().toLowerCase() : '';
  if (AUDIO.includes(ext)) {
    const stem = name.replace(/\.[^.]*$/, '').toLowerCase().replace(/[^a-z0-9-]+/g, '-').replace(/^-+|-+$/g, '') || 'son';
    let id = stem;
    for (let n = 2; existingSoundIds.includes(id); n++) id = `${stem}-${n}`;
    out.action = { type: 'Sound', id };
    out.sound = { id, path: p, volume: 100, on_repeat: 'Restart' };
  } else if (ext === 'exe') out.action = { type: 'Launch', program: p, args: [] };
  else if (ext === 'ps1') out.action = { type: 'Command', line: `& powershell -NoProfile -ExecutionPolicy Bypass -File ${q(p)}` };
  else if (ext === 'bat' || ext === 'cmd') out.action = { type: 'Command', line: `& ${q(p)}` };
  else out.action = { type: 'Open', target: p }; // .lnk inclus : ShellExecute sait l'ouvrir, pas CreateProcess
  return out;
}
