// Fichier depose sur une touche -> action. Pur : aucun acces DOM.
import { canBind } from './draft.js';

export const AUDIO = ['mp3', 'wav', 'ogg', 'flac'];

// Message de refus pour une entree non liable (natives, M1–M3, molette gauche), sinon null.
export function refusal(target, profileId, natives) {
  if (canBind(profileId, target, natives)) return null;
  if (/^M[123]$/.test(target)) return 'Touche de profil du firmware : non modifiable ici';
  if (target.startsWith('ENC_L_')) return 'Molette gauche du firmware : non modifiable ici';
  return 'Touche native du firmware : modifiable plus tard depuis le pad';
}

// SoundDef d'un fichier audio, id tire du nom et rendu unique.
export function soundFromPath(p, existingSoundIds = []) {
  const stem = p.split(/[\\/]/).pop().replace(/\.[^.]*$/, '').toLowerCase().replace(/[^a-z0-9-]+/g, '-').replace(/^-+|-+$/g, '') || 'son';
  let id = stem;
  for (let n = 2; existingSoundIds.includes(id); n++) id = `${stem}-${n}`;
  return { id, path: p, volume: 100, on_repeat: 'Restart' };
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
    out.sound = soundFromPath(p, existingSoundIds);
    out.action = { type: 'Sound', id: out.sound.id };
  } else if (ext === 'exe') out.action = { type: 'Launch', program: p, args: [] };
  // Chemin = entree argv separee : rien ne le reinterprete (std::process::Command passe les .bat par cmd.exe avec son echappement).
  else if (ext === 'ps1') out.action = { type: 'Launch', program: 'powershell.exe', args: ['-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', p] };
  else if (ext === 'bat' || ext === 'cmd') out.action = { type: 'Launch', program: p, args: [] };
  else out.action = { type: 'Open', target: p }; // .lnk inclus : ShellExecute sait l'ouvrir, pas CreateProcess
  return out;
}
