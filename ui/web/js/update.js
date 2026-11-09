// Logique pure des mises a jour (aucun acces DOM ni stockage).

export const EVERY = 6 * 3600 * 1000;

// Au demarrage (jamais verifie), puis toutes les 6 h ; jamais si le reglage est coupe.
// Une date illisible ou dans le futur (horloge reculee) compte comme « jamais ».
export const shouldCheck = (lastCheck, now, enabled) =>
  !!enabled && (!Number.isFinite(lastCheck) || lastCheck > now || now - lastCheck >= EVERY);

const MESSAGES = {
  network: 'Mise à jour impossible : pas de connexion au serveur',
  config: 'Mises à jour non configurées dans cette version',
  signature: 'Mise à jour refusée : signature invalide',
  engine: 'Le moteur ne s’est pas arrêté : mise à jour annulée',
  install: 'Installation de la mise à jour impossible',
};

// Erreur d'update_check / update_install (« code: detail ») -> phrase courte.
export function describe(err) {
  const s = String(err?.message ?? err);
  return MESSAGES[s.split(':')[0]] ?? `Mise à jour impossible : ${s}`;
}

// Evenement update-progress -> { pct (null sans taille connue), text }.
export function progressText(downloaded, total) {
  if (!total) return { pct: null, text: `${(downloaded / 1e6).toFixed(1)} Mo` };
  const pct = Math.min(100, Math.floor((downloaded / total) * 100));
  return { pct, text: pct >= 100 ? 'Installation…' : `${pct} %` };
}
