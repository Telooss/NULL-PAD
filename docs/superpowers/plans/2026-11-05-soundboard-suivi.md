# Soundboard — points de suivi

Relevés par les relectures (2026-11-06), jugés non bloquants. Le tri de la revue finale les classe « peut rester ».

## À vérifier sur la vraie machine (décision d'Alan)

- Veille Windows : avec au moins un son configuré, le casque et VB-Cable restent ouverts en continu. Vérifier `powercfg /requests` moteur lancé ; si le moteur y figure, choisir entre fermer les sorties après inactivité (premier appui plus lent) ou documenter.
- VB-Cable en détection auto installé pendant que le moteur tourne : pas repris avant un redémarrage du moteur ou une sauvegarde de la config.

## Mineurs différés

- **Task 1** : pas de test SoundVolume delta -101 ; mic_mode "App" jamais parse en test ; volume >255 rejete en Json et non Invalid ; message "duplicate son" melange EN/FR
- **Task 2** : tests manquants clip vide / drop pendant fondu / stop avant 1er echantillon / master pendant fondu ; fondu nul sous 200 Hz
- **Task 3** : erreur de decodage en cours de flux = clip tronque accepte (limite rodio) ; echec non mis en cache (double warn si meme chemin casse) ; Vec->Arc copie (pic memoire x2 sur 5 min) ; tests: early stop non prouve, "fichier vide" et elagage du cache non testes, fichiers temp fuient si assert echoue
- **Task 4** : apply stocke bibliotheque puis reglages (fenetre breve incoherente) ; voix d'ids retires jamais elaguees ; change_mic_gain load+store non atomique (un seul appelant: worker) ; lock().unwrap() panique si mutex empoisonne
- **Task 4** : indices exacts 4/8/12 de master_scales_playing_sound lies a la cadence du mixeur
- **Task 5** : never_returns_none ne couvre pas amorce->vide->producteur lache ; tolerance abs_diff<=2 du test de derive ; samples_for=0 pour taux minuscule
- **Task 5** : push_frames panique si channels == 0 (theorique) ; unwrap_or(0.0) devenu inatteignable dans next()
- **Task 6** : taille de tampon journalisee = BufferSize::Default, pas un nombre ; micro App impossible a ouvrir (format non F32) reessaye toutes les 5 s pour toujours ; thread jamais termine (accepte)
- **Task 7** : test on_reload sensible a un 2e evenement apres le debounce ; panique dans on_reload tuerait le thread du watcher ; bras MicVolume/MicMute mode App non testes
- **Final review** : StreamError sans generation (peut fermer une fois un flux neuf, borne par 1 s) ; sleep 1 s bloque Apply ; double log sans sortie ; micro mort non rouvert si le thread d'entree meurt sur une erreur "warn"
