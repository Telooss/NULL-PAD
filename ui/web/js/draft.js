// Brouillon d'un profil : rien n'est ecrit avant « Enregistrer ». Pur : aucun acces DOM.
export function canBind(profileId, input, natives = {}) {
  if (/^M[123]$/.test(input) || input.startsWith('ENC_L_')) return false;
  return !(profileId === 1 && input in natives);
}

// JSON a cles triees : comparaison independante de l'ordre des cles et des liaisons.
const canon = (v) => JSON.stringify(v, (_, x) => (x && typeof x === 'object' && !Array.isArray(x)
  ? Object.fromEntries(Object.keys(x).sort().map((k) => [k, x[k]])) : x));
const norm = (p) => canon({ name: p.name, tag: p.tag,
  bindings: [...p.bindings].sort((a, b) => (a.input + a.trigger).localeCompare(b.input + b.trigger)) });

// Config la plus recente + le profil edite + les sons ajoutes/modifies : le reste n'est jamais ecrase.
export function mergeDraft(latest, profile, soundChanges = []) {
  const out = structuredClone(latest);
  const i = out.profiles.findIndex((p) => p.id === profile.id);
  if (i < 0) throw new Error(`profil ${profile.id} supprimé entre-temps`);
  out.profiles[i] = structuredClone(profile);
  if (soundChanges.length) {
    const lib = out.sound?.sounds ?? [];
    for (const s of soundChanges) {
      const j = lib.findIndex((x) => x.id === s.id);
      if (j >= 0) lib[j] = structuredClone(s); else lib.push(structuredClone(s));
    }
    out.sound = { ...out.sound, sounds: lib };
  }
  return out;
}

export function createDraft(config, profileId) {
  const base = structuredClone(config);
  const idx = base.profiles.findIndex((p) => p.id === profileId);
  if (idx < 0) throw new Error(`profil ${profileId} inconnu`);
  const start = norm(base.profiles[idx]);
  // Bibliotheque de sons (globale) : ajouts par depot, volume et rappui edites.
  const lib = structuredClone(base.sound?.sounds ?? []);
  const libStart = canon(lib);
  const cur = structuredClone(base.profiles[idx]);
  return {
    get: () => cur,
    // `binding` : une Binding complete ({input, trigger, action, label?}) ou null pour retirer.
    set(input, trigger, binding) {
      const i = cur.bindings.findIndex((b) => b.input === input && b.trigger === trigger);
      if (binding === null) { if (i >= 0) cur.bindings.splice(i, 1); return; }
      const nb = binding.action ? { ...binding, input, trigger } : { input, trigger, action: binding };
      if (i >= 0) cur.bindings[i] = nb; else cur.bindings.push(nb);
    },
    // SoundDef issu d'un depot de fichier ; ignore si l'id existe deja.
    addSound(def) {
      if (!lib.some((s) => s.id === def.id)) lib.push(structuredClone(def));
    },
    updateSound(id, patch) {
      const s = lib.find((x) => x.id === id);
      if (s) Object.assign(s, patch);
    },
    sounds: () => lib,
    // Sons ajoutes ou modifies depuis l'entree en configuration.
    soundChanges: () => lib.filter((s) => canon(base.sound?.sounds?.find((x) => x.id === s.id)) !== canon(s)),
    setProfileMeta({ name, tag }) { cur.name = name; cur.tag = tag; },
    dirty: () => norm(cur) !== start || canon(lib) !== libStart,
    merged() {
      const out = structuredClone(base);
      out.profiles[idx] = structuredClone(cur);
      if (canon(lib) !== libStart) out.sound = { ...out.sound, sounds: structuredClone(lib) };
      return out;
    },
  };
}
