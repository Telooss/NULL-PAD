// Logique pure du mode vue (aucun acces DOM) : descriptions, legendes, modele du panneau, textes.
import { ACTIONS, TRIGGERS, TRIGGER_LABELS } from './catalog.js';
import { legendFor, fitLegend } from './legend.js';

export const esc = (s) => String(s).replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[c]);

// Types hors catalogue -> entree du catalogue qui les couvre.
const ALIAS = { Open: 'Launch', VolumeMute: 'Volume', MicVolume: 'Volume', MicMute: 'Volume', SoundVolume: 'Sound', SoundMute: 'Sound', StopSounds: 'Sound' };
const entryOf = (type) => ACTIONS.find((a) => a.type === (ALIAS[type] ?? type) || a.types?.includes(type));
const sign = (d) => `${d < 0 ? '−' : '+'}${Math.abs(d)}`;
const clip = (s, n = 48) => (s.length > n ? `${s.slice(0, n - 1)}…` : s);

function detail(a) {
  switch (a.type) {
    case 'Shortcut': return a.keys;
    case 'Text': return clip(a.text);
    case 'Macro': return `${a.steps.length} étape${a.steps.length > 1 ? 's' : ''}`;
    case 'Launch': return clip([a.program, ...(a.args ?? [])].join(' '));
    case 'Open': return clip(a.target);
    case 'Command': return clip(a.line);
    case 'Media': return { PlayPause: 'Lecture / pause', Next: 'Piste suivante', Prev: 'Piste précédente', Stop: 'Stop' }[a.key] ?? a.key;
    case 'Volume': return `Volume Windows ${sign(a.delta)}`;
    case 'VolumeMute': return 'Couper le son de Windows';
    case 'MicVolume': return `Volume du micro ${sign(a.delta)}`;
    case 'MicMute': return 'Couper le micro';
    case 'Sound': return a.id;
    case 'StopSounds': return 'Arrêter les sons';
    case 'SoundVolume': return `Volume des sons ${sign(a.delta)}`;
    case 'SoundMute': return 'Couper les sons';
    case 'ObsScene': return `Scène : ${a.scene}`;
    case 'ObsMute': return `Couper : ${a.input}`;
    case 'ObsStream': return `Stream : ${a.op}`;
    case 'ObsRecord': return `Enregistrement : ${a.op}`;
    case 'ObsReplaySave': return 'Sauver le replay';
    case 'MidiNote': return `Canal ${a.channel} · note ${a.note} · vélocité ${a.velocity}`;
    case 'MidiCc': return `Canal ${a.channel} · CC ${a.cc} · valeur ${a.value}`;
    case 'Toggle': return `${describeAction(a.on).title} ⇄ ${describeAction(a.off).title}`;
    default: return '';
  }
}

export function describeAction(a) {
  const e = entryOf(a.type);
  return { title: e?.label ?? a.type, desc: detail(a), icon: e?.icon ?? 'zap' };
}

// Les natives ne valent que sur le profil 1 ; M1–M3 et les molettes valent partout.
export const nativesFor = (natives, profileId) =>
  profileId === 1 ? natives : Object.fromEntries(Object.entries(natives).filter(([k]) => /^(M\d|ENC_)/.test(k)));

export const profileOf = (config, id) => config?.profiles.find((p) => p.id === id);

// M1–M3 = profils 1 a 3.
export const profileSlots = (config) => [1, 2, 3].map((id) => {
  const p = profileOf(config, id);
  return p ? { id, name: p.name, tag: p.tag } : null;
});

export const INPUTS = [...Array.from({ length: 20 }, (_, i) => `K${String(i).padStart(2, '0')}`), 'M1', 'M2', 'M3',
  'ENC_L_CW', 'ENC_L_CCW', 'ENC_L_PRESS', 'ENC_R_CW', 'ENC_R_CCW', 'ENC_R_PRESS'];

// InputId -> { text: [l1, l2], kind }
export function legendsFor(config, profileId, natives) {
  const prof = profileOf(config, profileId);
  const nat = nativesFor(natives, profileId);
  const out = {};
  for (const input of INPUTS) {
    const l = legendFor(prof?.bindings.find((b) => b.input === input && b.trigger === 'Press'), nat, config?.sound?.sounds, input);
    out[input] = { text: fitLegend(l.text), kind: l.kind };
  }
  return out;
}

export function placeOf(input) {
  const k = /^K(\d\d)$/.exec(input)?.[1];
  if (k) return `Rangée ${Math.floor(+k / 4) + 1} · colonne ${(+k % 4) + 1}`;
  if (/^M[123]$/.test(input)) return `Touche de profil ${input}`;
  const e = /^ENC_([LR])_/.exec(input)?.[1];
  return e ? `Molette ${e === 'L' ? 'gauche' : 'droite'}` : '';
}

const nativeNote = (legend) => `Touche native · ${legend}, envoyée par le pad, marche sans nullctl`;
const ENC_PARTS = [['CW', 'Sens horaire'], ['CCW', 'Sens anti-horaire'], ['PRESS', 'Clic']];

// Modele du panneau lecture seule : { name, place, kind, note?, sections: [{ title?, native?, rows: [{ trigger, label, action|null }] }] }
export function panelModel(config, profileId, input, natives) {
  const prof = profileOf(config, profileId);
  const nat = nativesFor(natives, profileId);
  const rows = (inp, triggers) => triggers.map((t) => {
    const b = prof?.bindings.find((x) => x.input === inp && x.trigger === t);
    return { trigger: t, label: TRIGGER_LABELS[t], action: b ? { ...describeAction(b.action), label: b.label } : null };
  });
  const place = placeOf(input);
  if (/^M[123]$/.test(input)) return { name: input, place, kind: 'profile', note: 'Touche de profil du firmware', sections: [] };
  const enc = /^ENC_([LR])_/.exec(input)?.[1];
  if (enc) {
    const sections = ENC_PARTS.map(([part, title]) => {
      const inp = `ENC_${enc}_${part}`;
      return inp in nat ? { title, native: nativeNote(nat[inp]), rows: [] } : { title, rows: rows(inp, part === 'PRESS' ? TRIGGERS : ['Press']) };
    });
    return { name: `Molette ${enc === 'L' ? 'gauche' : 'droite'}`, place, kind: 'encoder', sections };
  }
  if (input in nat) return { name: nat[input], place, kind: 'native', note: nativeNote(nat[input]), sections: [] };
  const l = legendFor(prof?.bindings.find((b) => b.input === input && b.trigger === 'Press'), nat, config?.sound?.sounds, input);
  return { name: l.kind === 'free' ? input : l.text, place, kind: 'key', sections: [{ rows: rows(input, TRIGGERS) }] };
}

// Ligne de scene d'un evenement `action`. `binding` : la liaison correspondante, si connue.
export function toastText(e, binding) {
  if (!e.ok) return `${e.input} → erreur : ${e.error ?? 'échec'}`;
  const d = binding ? describeAction(binding.action) : { title: e.kind, desc: '' };
  return `${e.input} → ${d.title}${d.desc ? ` : ${d.desc}` : ''}`;
}

export const linkText = (link) => ({ usb: 'USB', ble: 'BLE' })[link] ?? 'Pad absent';
