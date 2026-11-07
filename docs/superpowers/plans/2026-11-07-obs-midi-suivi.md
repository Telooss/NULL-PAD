# OBS + MIDI — points de suivi

Relevés par les relectures (2026-11-07), non bloquants.

## À vérifier avec le vrai OBS / Ableton
- Scène, mute, stream, enregistrement, replay ; OBS fermé puis rouvert ; mauvais mot de passe.
- Note et CC reçus dans Ableton ; molette en mode « Relative (2's Comp.) » ; loopMIDI fermé puis relancé.

## Mineurs
- Les appuis OBS faits pendant qu'OBS est injoignable sont jetés, jamais rejoués (voulu) ; une erreur sur une connexion déjà ouverte ne vide pas la file.
- `localhost` peut doubler le temps de connexion (::1 puis 127.0.0.1) ; la résolution DNS n'est pas bornée. Garder `127.0.0.1`.
- `scene` / `input` faits d'espaces acceptés par la config (OBS refuse et le moteur journalise).
- Verrou MIDI tenu pendant l'énumération des ports (quelques ms, seulement à la (re)connexion).
- Le test de routage du dispatcher ne prouve que la transmission ; le branchement réel tient au `match` exhaustif.
- Les tests OBS dépendent d'un refus de connexion lent (~2 s sous Windows) pour discriminer.
