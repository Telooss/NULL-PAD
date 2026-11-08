// Logique pure de l'ecran Reglages (aucun acces DOM).
import { soundFromPath } from './drop.js';

export const TIMING_MIN = 50;
export const TIMING_MAX = 5000;
export const SECTIONS = ['sound', 'obs', 'midi', 'timing'];

const clampInt = (v, lo, hi, fallback) => {
  const n = Math.round(Number(v));
  return Number.isFinite(n) ? Math.min(hi, Math.max(lo, n)) : fallback;
};
export const clampTiming = (v, fallback = 300) => clampInt(v, TIMING_MIN, TIMING_MAX, fallback);
export const clampPort = (v, fallback = 4455) => clampInt(v, 1, 65535, fallback);
export const clampPercent = (v, fallback = 100) => clampInt(v, 0, 100, fallback);

// Liaisons qui jouent le son `id` (y compris dans un Toggle ou une macro) -> [{ profile, input }].
export function soundInUse(config, id) {
  const uses = (a) => !!a && typeof a === 'object' && (
    (a.type === 'Sound' && a.id === id) || Object.values(a).some((v) => (Array.isArray(v) ? v.some(uses) : uses(v))));
  return (config.profiles ?? []).flatMap((p) => p.bindings.filter((b) => uses(b.action)).map((b) => ({ profile: p.name, input: b.input })));
}

// Message de refus de retrait ; null si le son est libre.
export function inUseMessage(config, id) {
  const u = soundInUse(config, id);
  if (!u.length) return null;
  const keys = [...new Set(u.map((x) => `${x.input} (${x.profile})`))];
  return `« ${id} » est utilisé par ${keys.join(', ')} : changez ces touches d’abord`;
}

export const settingsOf = (cfg) => structuredClone(Object.fromEntries(SECTIONS.map((k) => [k, cfg[k]])));

// Config la plus recente + les 4 sections du brouillon : les profils ne sont jamais ecrases.
export const mergeSettings = (latest, draft) => ({ ...structuredClone(latest), ...structuredClone(Object.fromEntries(SECTIONS.map((k) => [k, draft[k]]))) });

// Brouillon -> forme ecrite : mot de passe vide = aucun, bornes appliquees.
export function normalize(d) {
  const o = structuredClone(d);
  o.obs.password = o.obs.password || null;
  o.obs.port = clampPort(o.obs.port);
  o.timing.hold_ms = clampTiming(o.timing.hold_ms);
  o.timing.double_ms = clampTiming(o.timing.double_ms);
  return o;
}

export const isDirty = (draft, base) => JSON.stringify(normalize(draft)) !== JSON.stringify(normalize(base));

// Ajoute un fichier audio a la bibliotheque (draft.sound.sounds) ; renvoie la definition.
export function addSoundFile(draft, path) {
  const def = soundFromPath(path, draft.sound.sounds.map((s) => s.id));
  draft.sound.sounds.push(def);
  return def;
}
