// Editeurs d'action : champs <-> action (forme de engine/src/config.rs). Pur : aucun acces DOM.
// Champ : { key, kind, label, value, options?, min?, max? }
// kinds : seg, text, textarea, lines, number, check, file, keys, slider, sound, steps, sub

// Types regroupes sous une entree du catalogue (le premier est celui du catalogue).
export const FAMILIES = {
  Launch: ['Launch', 'Open'],
  Sound: ['Sound', 'StopSounds', 'SoundVolume', 'SoundMute'],
  ObsScene: ['ObsScene', 'ObsMute', 'ObsStream', 'ObsRecord', 'ObsReplaySave'],
  Volume: ['Volume', 'VolumeMute', 'MicVolume', 'MicMute'],
  MidiNote: ['MidiNote', 'MidiCc'],
};
export const TYPE_LABELS = {
  Shortcut: 'Raccourci clavier', Text: 'Texte', Macro: 'Macro', Launch: 'Lancer', Open: 'Ouvrir', Command: 'Commande', Media: 'Média',
  Volume: 'Volume Windows', VolumeMute: 'Couper Windows', MicVolume: 'Volume micro', MicMute: 'Couper micro',
  Sound: 'Jouer', StopSounds: 'Tout arrêter', SoundVolume: 'Volume sons', SoundMute: 'Couper sons',
  ObsScene: 'Scène', ObsMute: 'Source', ObsStream: 'Stream', ObsRecord: 'Enreg.', ObsReplaySave: 'Replay',
  MidiNote: 'Note', MidiCc: 'CC', Toggle: 'Bascule',
};
// Types proposes dans une bascule ou une etape de macro : pas de bascule dedans.
export const NESTED_TYPES = Object.keys(TYPE_LABELS).filter((t) => t !== 'Toggle');

export const familyOf = (type) => Object.values(FAMILIES).find((f) => f.includes(type)) ?? [type];

export function defaultAction(type, sounds = []) {
  switch (type) {
    case 'Shortcut': return { type, keys: '' };
    case 'Text': return { type, text: '' };
    case 'Macro': return { type, steps: [] };
    case 'Launch': return { type, program: '', args: [] };
    case 'Open': return { type, target: '' };
    case 'Command': return { type, line: '' };
    case 'Media': return { type, key: 'PlayPause' };
    case 'Volume': case 'MicVolume': return { type, delta: 2 };
    case 'SoundVolume': return { type, delta: 10 };
    case 'Sound': return { type, id: sounds[0]?.id ?? '' };
    case 'ObsScene': return { type, scene: '' };
    case 'ObsMute': return { type, input: '' };
    case 'ObsStream': case 'ObsRecord': return { type, op: 'Toggle' };
    case 'MidiNote': return { type, channel: 1, note: 60, velocity: 100 };
    case 'MidiCc': return { type, channel: 1, cc: 1, value: 127 };
    case 'Toggle': return { type, on: { type: 'MicMute' }, off: { type: 'MicMute' } };
    default: return { type };
  }
}

const opts = (pairs) => pairs.map(([value, label]) => ({ value, label }));
const OPS = opts([['Start', 'Démarrer'], ['Stop', 'Arrêter'], ['Toggle', 'Basculer']]);
const num = (key, label, value, min, max) => ({ key, kind: 'number', label, value, min, max });
const delta = (a) => [num('delta', 'Pas (%)', a.delta, -100, 100)];

export function fieldsOf(a, sounds = []) {
  const fam = familyOf(a.type);
  const out = fam.length > 1 && fam[0] !== 'Launch'
    ? [{ key: 'type', kind: 'seg', label: 'Type', value: a.type, options: fam.map((t) => ({ value: t, label: TYPE_LABELS[t] })) }] : [];
  const def = sounds.find((s) => s.id === a.id);
  switch (a.type) {
    case 'Shortcut': out.push({ key: 'keys', kind: 'keys', label: 'Raccourci (appuyer sur les touches)', value: a.keys ?? '' }); break;
    case 'Text': out.push({ key: 'text', kind: 'textarea', label: 'Texte tapé', value: a.text ?? '' }); break;
    case 'Macro': out.push({ key: 'steps', kind: 'steps', label: 'Étapes', value: a.steps ?? [] }); break;
    case 'Launch': out.push(
      { key: 'program', kind: 'file', label: 'Programme', value: a.program ?? '' },
      { key: 'args', kind: 'lines', label: 'Arguments (un par ligne)', value: (a.args ?? []).join('\n') },
      { key: 'open', kind: 'check', label: 'URL ou fichier à ouvrir', value: false }); break;
    case 'Open': out.push(
      { key: 'target', kind: 'file', label: 'URL ou fichier', value: a.target ?? '' },
      { key: 'open', kind: 'check', label: 'URL ou fichier à ouvrir', value: true }); break;
    case 'Command': out.push({ key: 'line', kind: 'text', label: 'Ligne de commande', value: a.line ?? '' }); break;
    case 'Media': out.push({ key: 'key', kind: 'seg', label: 'Touche', value: a.key,
      options: opts([['PlayPause', 'Lecture'], ['Prev', 'Préc.'], ['Next', 'Suiv.'], ['Stop', 'Stop']]) }); break;
    case 'Volume': case 'MicVolume': case 'SoundVolume': out.push(...delta(a)); break;
    case 'Toggle': out.push({ key: 'on', kind: 'sub', label: 'Allumé', value: a.on }, { key: 'off', kind: 'sub', label: 'Éteint', value: a.off }); break;
    case 'Sound':
      out.push({ key: 'id', kind: 'sound', label: 'Son', value: a.id ?? '', options: sounds.map((s) => ({ value: s.id, label: s.id, path: s.path })) });
      if (def) out.push({ key: 'volume', kind: 'slider', label: 'Volume du son', value: def.volume ?? 100, min: 0, max: 100 },
        { key: 'on_repeat', kind: 'seg', label: 'Si on rappuie pendant le son', value: def.on_repeat ?? 'Restart', options: opts([['Restart', 'Relancer'], ['Stop', 'Couper']]) });
      break;
    case 'ObsScene': out.push({ key: 'scene', kind: 'text', label: 'Scène OBS', value: a.scene ?? '' }); break;
    case 'ObsMute': out.push({ key: 'input', kind: 'text', label: 'Source OBS', value: a.input ?? '' }); break;
    case 'ObsStream': case 'ObsRecord': out.push({ key: 'op', kind: 'seg', label: 'Opération', value: a.op, options: OPS }); break;
    case 'MidiNote': out.push(num('channel', 'Canal', a.channel, 1, 16), num('note', 'Note', a.note, 0, 127), num('velocity', 'Vélocité', a.velocity, 1, 127)); break;
    case 'MidiCc': out.push(num('channel', 'Canal', a.channel, 1, 16), num('cc', 'CC', a.cc, 0, 127), num('value', 'Valeur', a.value, 0, 127)); break;
  }
  return out;
}

// -> { action, soundPatch? } ; soundPatch = { id, patch } pour le SoundDef du brouillon.
export function applyField(a, key, value, sounds = []) {
  if (key === 'type') return { action: defaultAction(value, sounds) };
  if (key === 'open') return { action: value ? { type: 'Open', target: a.program ?? a.target ?? '' } : { type: 'Launch', program: a.target ?? a.program ?? '', args: [] } };
  if (key === 'args') return { action: { ...a, args: String(value).split(/\r?\n/).filter((l) => l.trim() !== '') } };
  if ((key === 'on' || key === 'off') && value?.type === 'Toggle') return { action: a }; // pas de bascule imbriquee
  const f = fieldsOf(a, sounds).find((x) => x.key === key);
  if (f?.kind === 'number' || f?.kind === 'slider') {
    const n = Math.round(Number(value));
    value = Number.isFinite(n) && String(value).trim() !== '' ? Math.min(f.max, Math.max(f.min, n)) : f.value;
  }
  if (a.type === 'Sound' && (key === 'volume' || key === 'on_repeat')) return { action: a, soundPatch: { id: a.id, patch: { [key]: value } } };
  return { action: { ...a, [key]: value } };
}

// Etapes de macro : Faire / Attendre.
export const addStep = (steps, kind) => [...steps, kind === 'Wait' ? { type: 'Wait', ms: 100 } : { type: 'Do', action: { type: 'Shortcut', keys: '' } }];
export const removeStep = (steps, i) => steps.filter((_, j) => j !== i);
export function moveStep(steps, i, d) {
  const j = i + d;
  if (j < 0 || j >= steps.length) return steps;
  const out = [...steps];
  [out[i], out[j]] = [out[j], out[i]];
  return out;
}
export const setStep = (steps, i, step) => steps.map((s, j) => (j === i ? step : s));

// Touche capturee -> nom compris par engine/src/actions/keys.rs ; null pour un modificateur seul.
const NAMED = { Enter: 'Enter', NumpadEnter: 'Enter', Escape: 'Esc', Tab: 'Tab', Space: 'Space', Backspace: 'Backspace', Delete: 'Delete',
  Home: 'Home', End: 'End', PageUp: 'PageUp', PageDown: 'PageDown', ArrowUp: 'Up', ArrowDown: 'Down', ArrowLeft: 'Left', ArrowRight: 'Right' };
export function shortcutOf(e) {
  const k = NAMED[e.code] ?? /^Key([A-Z])$/.exec(e.code)?.[1] ?? /^(?:Digit|Numpad)(\d)$/.exec(e.code)?.[1] ?? /^F([1-9]|1\d|2[0-4])$/.exec(e.code)?.[0];
  if (!k) return null;
  return [e.ctrlKey && 'Ctrl', e.shiftKey && 'Shift', e.altKey && 'Alt', e.metaKey && 'Win', k].filter(Boolean).join('+');
}
