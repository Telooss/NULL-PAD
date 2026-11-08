// Legendes affichees sur les touches. Pur : aucun acces DOM.
const base = (p) => p.split(/[\\/]/).filter(Boolean).pop() ?? p;
const stem = (p) => base(p).replace(/\.[^.]*$/, '');

function ofAction(a) {
  switch (a.type) {
    case 'Sound': return a.id.toUpperCase();
    case 'ObsScene': return a.scene;
    case 'Shortcut': return a.keys.toUpperCase();
    case 'Volume': return a.delta < 0 ? 'VOL−' : 'VOL+';
    case 'SoundVolume': return a.delta < 0 ? 'SON−' : 'SON+';
    case 'MicVolume': return a.delta < 0 ? 'MIC−' : 'MIC+';
    case 'Launch': return stem(a.program);
    case 'MidiNote': return `N${a.note}`;
    case 'MidiCc': return `CC${a.cc}`;
    case 'Text': return a.text.toUpperCase();
    case 'Open': return base(a.target);
    case 'Macro': return 'MACRO';
    case 'Command': return 'CMD';
    case 'VolumeMute': return 'MUTE';
    case 'MicMute': return 'MIC';
    case 'Media': return { PlayPause: 'PLAY', Next: 'NEXT', Prev: 'PREV', Stop: 'STOP' }[a.key] ?? 'MEDIA';
    case 'Toggle': return a.on ? ofAction(a.on) : 'BASCULE';
    default: return a.type.replace(/([a-z])([A-Z])/g, '$1 $2').toUpperCase();
  }
}

// `input` sert quand il n'y a pas de liaison.
export function legendFor(binding, natives = {}, sounds = [], input = binding?.input) {
  if (binding?.label) return { text: binding.label, kind: 'action' };
  if (binding?.action) return { text: ofAction(binding.action), kind: 'action' };
  if (input in natives) return { text: natives[input], kind: /^M[123]$/.test(input) ? 'profile' : 'native' };
  return { text: '+', kind: 'free' };
}

// 2 lignes de 6 caracteres au plus, coupees a un espace ou apres un « + ».
export function fitLegend(text, w = 6) {
  if (text.length <= w) return [text];
  let cut = w;
  for (let i = w; i > 0; i--) {
    if (text[i] === ' ' || text[i - 1] === '+') { cut = i; break; }
  }
  const l1 = text.slice(0, cut).trimEnd();
  const l2 = text.slice(cut).trim();
  return [l1, l2.length > w ? `${l2.slice(0, w - 1)}…` : l2];
}
