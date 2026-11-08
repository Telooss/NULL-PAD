// Catalogue des actions proposees dans le panneau (libelles et icones Lucide de la preview EcranPrincipal).
export const ACTIONS = [
  { type: 'Shortcut', label: 'Raccourci clavier', desc: 'Combinaison, touche média', icon: 'command' },
  { type: 'Text', label: 'Texte', desc: "Tape un texte d'un coup", icon: 'type' },
  { type: 'Macro', label: 'Macro', desc: "Suite d'actions avec délais", icon: 'list-ordered' },
  { type: 'Launch', label: 'Lancer', desc: 'Appli, fichier ou URL', icon: 'rocket' },
  { type: 'Command', label: 'Commande', desc: 'PowerShell, .bat, programme', icon: 'terminal' },
  { type: 'Sound', label: 'Son', desc: 'Soundboard, joué dans le micro', icon: 'volume-2' },
  { type: 'ObsScene', label: 'OBS', desc: 'Scène, source, stream, replay', icon: 'video',
    types: ['ObsScene', 'ObsMute', 'ObsStream', 'ObsRecord', 'ObsReplaySave'] },
  { type: 'Volume', label: 'Volume et micro', desc: 'Volume Windows, couper le micro', icon: 'sliders-horizontal' },
  { type: 'MidiNote', label: 'MIDI', desc: 'Note ou CC vers Ableton', icon: 'music', types: ['MidiNote', 'MidiCc'] },
  { type: 'Media', label: 'Média', desc: 'Lecture, piste suivante ou précédente', icon: 'play' },
  { type: 'Toggle', label: 'Bascule', desc: 'Alterne deux actions à chaque appui', icon: 'toggle-left' },
];

export const TRIGGERS = ['Press', 'Hold', 'DoubleTap'];
export const TRIGGER_LABELS = { Press: 'Appui', Hold: 'Appui long', DoubleTap: 'Double appui' };
