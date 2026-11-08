// Profils : aides pures (id libre, tag propose, ajout / retrait) et ecrans DOM (creation, menu d'un emplacement).

const ALPHABET = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789';
export const MAX_PROFILES = 8;
export const NAME_MAX = 18;

const fold = (s) => String(s).normalize('NFD').replace(/[̀-ͯ]/g, '').toUpperCase();

// A-Z0-9 en majuscules, 3 caracteres au plus (champ du tag).
export const cleanTag = (s) => fold(s).replace(/[^A-Z0-9]/g, '').slice(0, 3);

// Initiales des mots d'abord, puis les premieres lettres restantes, dans l'ordre du nom ; bourre par X.
export function suggestTag(name) {
  const words = fold(name).split(/[^A-Z0-9]+/).filter(Boolean);
  const flat = words.join('');
  const pick = [];
  let off = 0;
  for (const w of words) { pick.push(off); off += w.length; }
  pick.splice(3);
  for (let i = 0; i < flat.length && pick.length < 3; i++) if (!pick.includes(i)) pick.push(i);
  return pick.sort((a, b) => a - b).map((i) => flat[i]).join('').padEnd(3, 'X');
}

export function nextFreeId(config) {
  const used = new Set((config?.profiles ?? []).map((p) => p.id));
  for (let id = 1; id <= MAX_PROFILES; id++) if (!used.has(id)) return id;
  return null;
}

export function withNewProfile(config, { name, tag, copyFrom }) {
  const id = nextFreeId(config);
  if (id === null) throw new Error(`${MAX_PROFILES} profils au maximum`);
  const src = copyFrom == null ? null : config.profiles.find((p) => p.id === copyFrom);
  const out = structuredClone(config);
  out.profiles.push({ id, name, tag, bindings: src ? structuredClone(src.bindings) : [] });
  out.profiles.sort((a, b) => a.id - b.id);
  return out;
}

export function withoutProfile(config, id) {
  if (id === 1) throw new Error('le profil 1 ne se supprime pas');
  const out = structuredClone(config);
  out.profiles = out.profiles.filter((p) => p.id !== id);
  return out;
}

// Barre des profils : M1-M3 (null = vide) puis les profils 4-8 existants (atteignables par ZMK Studio).
export function barSlots(config) {
  const ps = config?.profiles ?? [];
  return [1, 2, 3].map((id) => ps.find((p) => p.id === id) ?? null).concat(ps.filter((p) => p.id > 3).sort((a, b) => a.id - b.id));
}

export const slotLabel = (id) => (id <= 3 ? `M${id}` : `P${id}`);

// Petit menu contextuel sous `anchor` ; items : [[libelle, callback]]. Echap / clic ailleurs le ferme et rend le focus.
export function openMenu(anchor, items) {
  document.querySelector('.nc-menu')?.remove();
  const m = document.createElement('div');
  m.className = 'nc-menu';
  m.setAttribute('role', 'menu');
  const close = (refocus) => {
    m.remove();
    document.removeEventListener('keydown', key, true);
    document.removeEventListener('pointerdown', away, true);
    if (refocus) anchor.focus();
  };
  const key = (e) => {
    if (e.key === 'Escape') { e.stopPropagation(); close(true); }
    else if (e.key === 'Tab') { e.preventDefault(); close(true); }
  };
  const away = (e) => { if (!m.contains(e.target)) close(false); };
  for (const [label, cb] of items) {
    const b = document.createElement('button');
    b.className = 'nc-btn';
    b.setAttribute('role', 'menuitem');
    b.textContent = label;
    b.onclick = () => { close(false); cb(); };
    m.appendChild(b);
  }
  const r = anchor.getBoundingClientRect();
  m.style.left = `${Math.round(r.left)}px`;
  m.style.top = `${Math.round(r.bottom + 4)}px`;
  document.body.appendChild(m);
  document.addEventListener('keydown', key, true);
  document.addEventListener('pointerdown', away, true);
  m.querySelector('button').focus();
}

// Ecran de creation plein cadre (CreationProfil). id : profil a creer ; from : profil affiche (copie), ou null.
// onPreview({ tag, copy }) : apercu en direct sur le pad. -> { name, tag, copy } ou null (annule).
export function openCreate(host, { id, from, onPreview }) {
  const N = window.NullCtl;
  return new Promise((resolve) => {
    let tag = ['X', 'X', 'X'];
    let touched = false;
    let copy = false;
    const wrap = document.createElement('div');
    wrap.className = 'cp-overlay';
    wrap.innerHTML =
      '<div class="cp-left"><span class="cp-ghost"></span>' +
      '<div class="nc-w98" style="left:24px;top:22px;width:250px;transform:rotate(-2deg);z-index:2"><div class="bar">nouveau_profil.exe<i>×</i></div>' +
      '<div class="body"><span class="cp-w98"></span><div class="prog"><b></b><b></b><b></b><b></b><b></b><b></b><b></b><b></b><b></b></div></div></div>' +
      '<i class="nc-tape" style="left:190px;top:12px;transform:rotate(9deg);z-index:3"></i>' +
      '<div class="nc-w98" style="left:24px;bottom:34px;width:180px;transform:rotate(2.5deg);z-index:2"><div class="bar">ZMK.SYS<i>×</i></div>' +
      '<div class="body"><span>3 caractères max. sur l’écran. Pas d’accents.</span><span class="ok">OK</span></div></div></div>' +
      '<section class="cp-right" role="dialog" aria-modal="true" aria-label="Nouveau profil">' +
      '<div class="nc-label cp-step"></div>' +
      '<div><label class="nc-label" for="cp-name" style="display:block;margin-bottom:6px">Nom du profil</label>' +
      `<input class="cp-name" id="cp-name" maxlength="${NAME_MAX}" spellcheck="false" autocomplete="off">` +
      `<div class="cp-help" style="margin-top:8px">Affiché en haut de nullctl. Jusqu’à ${NAME_MAX} caractères.</div></div>` +
      '<div class="cp-row"><div><div class="nc-label" style="margin-bottom:8px">Tag · affiché sur l’écran du pad</div><div class="cp-tag"></div>' +
      '<div class="cp-help" style="margin-top:8px">Proposé depuis le nom. A–Z et 0–9.</div></div>' +
      '<div><div class="nc-label" style="margin-bottom:8px">Touche de profil</div><div class="cp-slots"></div><div class="cp-help cp-mnote" style="margin-top:8px"></div></div></div>' +
      '<div><div class="nc-label" style="margin-bottom:8px">Point de départ</div><div class="cp-opts">' +
      '<button class="cp-opt is-on" data-copy="0" aria-pressed="true"><b><i data-lucide="square-dashed"></i>Vide</b><span>Toutes les touches libres</span></button>' +
      '<button class="cp-opt" data-copy="1" aria-pressed="false"><b><i data-lucide="copy"></i><span class="cp-copyname"></span></b><span>Mêmes réglages, à modifier</span></button></div></div>' +
      '<div class="cp-go"><button class="nc-btn is-primary cp-create"><i data-lucide="corner-down-left"></i>Créer le profil <kbd>Entrée</kbd></button>' +
      '<button class="nc-btn cp-cancel">Annuler <kbd>Échap</kbd></button></div></section>';

    const q = (s) => wrap.querySelector(s);
    const name = q('.cp-name');
    q('.cp-ghost').textContent = String(id).padStart(2, '0');
    q('.cp-w98').textContent = `Écriture de la couche ${id} dans le NULL/PAD…`;
    q('.cp-step').textContent = `Nouveau profil · ${id <= 3 ? `emplacement M${id}` : 'sans touche M'} · couche ZMK ${id} sur ${MAX_PROFILES}`;
    q('.cp-mnote').textContent = id <= 3 ? `La touche M${id} est fixée par le firmware : rien à régler.` : 'Pas de touche M : atteignable par ZMK Studio.';
    const slots = q('.cp-slots');
    for (const m of [1, 2, 3]) {
      const b = document.createElement('button');
      b.className = `nc-slot${m === id ? ' is-active' : ''}`;
      b.disabled = true;
      b.innerHTML = `<span class="nc-m">M${m}</span>`;
      slots.appendChild(b);
    }
    const copyBtn = q('[data-copy="1"]');
    if (from) q('.cp-copyname').textContent = `Copie de ${from.name}`; else copyBtn.hidden = true;

    const go = q('.cp-create');
    const preview = () => onPreview({ tag: tag.join(''), copy });
    const drawTag = () => {
      const box = q('.cp-tag');
      box.textContent = '';
      tag.forEach((ch, i) => {
        const d = document.createElement('div');
        d.className = 'cp-letter';
        d.innerHTML = '<button aria-label="Lettre suivante"><i data-lucide="chevron-up"></i></button><output></output><button aria-label="Lettre précédente"><i data-lucide="chevron-down"></i></button>';
        d.querySelector('output').textContent = ch;
        d.querySelectorAll('button').forEach((b, up) => {
          b.onclick = () => {
            tag[i] = ALPHABET[(ALPHABET.indexOf(tag[i]) + (up ? 1 : ALPHABET.length - 1)) % ALPHABET.length];
            touched = true;
            drawTag();
            q('.cp-tag').children[i].querySelectorAll('button')[up].focus();
          };
        });
        box.appendChild(d);
      });
      N.icons();
      preview();
    };
    name.oninput = () => {
      if (!touched) { tag = suggestTag(name.value).split(''); drawTag(); }
      go.disabled = !name.value.trim();
    };
    wrap.querySelectorAll('.cp-opt').forEach((b) => {
      b.onclick = () => {
        copy = b.dataset.copy === '1';
        wrap.querySelectorAll('.cp-opt').forEach((o) => { o.classList.toggle('is-on', o === b); o.setAttribute('aria-pressed', String(o === b)); });
        preview();
      };
    });

    const done = (v) => { wrap.remove(); document.removeEventListener('keydown', key, true); resolve(v); };
    const submit = () => { if (name.value.trim()) done({ name: name.value.trim(), tag: tag.join(''), copy }); };
    const key = (e) => {
      if (e.key === 'Escape') { e.stopPropagation(); done(null); }
      else if (e.key === 'Enter' && e.target.tagName !== 'BUTTON') { e.preventDefault(); submit(); }
    };
    go.onclick = submit;
    q('.cp-cancel').onclick = () => done(null);
    document.addEventListener('keydown', key, true);
    host.appendChild(wrap);
    go.disabled = true;
    drawTag();
    name.focus();
  });
}
