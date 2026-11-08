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

export function createDraft(config, profileId) {
  const base = structuredClone(config);
  const idx = base.profiles.findIndex((p) => p.id === profileId);
  if (idx < 0) throw new Error(`profil ${profileId} inconnu`);
  const start = norm(base.profiles[idx]);
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
    setProfileMeta({ name, tag }) { cur.name = name; cur.tag = tag; },
    dirty: () => norm(cur) !== start,
    merged() {
      const out = structuredClone(base);
      out.profiles[idx] = structuredClone(cur);
      return out;
    },
  };
}
