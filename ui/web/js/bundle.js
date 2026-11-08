/* @ds-bundle: {"format":4,"namespace":"NullCtl","components":[]} */
/* Aides partagées par les maquettes : coque de la fenêtre, pad 3D cliquable, fil touche → panneau, glitch. */
(function () {
  var VIOLET_KEYS = { F20: 1, F21: 1, F22: 1, F23: 1 };

  // Profil « Default » : le pavé numérique (touches natives). '' = touche libre.
  var NUMPAD = ['Num', '/', '*', '−', '7', '8', '9', '+', '4', '5', '6', '', '1', '2', '3', 'Ent', '0', '00', '.', '', 'DEF', '', ''];

  function place(F) { // nom lisible d'une touche : rangée / colonne, ou M1–M3
    var n = +F.slice(1);
    if (n > 20) return 'Touche de profil M' + (n - 20);
    return 'Rangée ' + (Math.ceil(n / 4)) + ' · colonne ' + (((n - 1) % 4) + 1);
  }

  function shell(o) {
    var slots = o.profiles.map(function (p, i) {
      var m = 'M' + (i + 1);
      if (!p) return '<button class="nc-slot" data-slot="' + i + '"><span class="nc-m">' + m + '</span><span>Vide</span><i data-lucide="plus" style="margin-left:auto"></i></button>';
      return '<button class="nc-slot' + (i === o.active ? ' is-active' : '') + '" data-slot="' + i + '"><span class="nc-m">' + m + '</span>' +
        '<span class="nc-sticker">' + p.name + '</span><span class="nc-tag">' + p.tag + '</span></button>';
    }).join('');
    return '<header class="nc-titlebar"><span class="nc-wordmark">null<b>ctl</b><i></i></span>' +
      '<span class="nc-sub">— NULL/PAD</span><span class="nc-winctl"><span><i data-lucide="minus"></i></span><span><i data-lucide="square"></i></span><span><i data-lucide="x"></i></span></span></header>' +
      '<nav class="nc-topbar"><span class="nc-label">Profils</span>' + slots +
      '<button class="nc-newprofile"><i data-lucide="plus"></i>Nouveau profil</button>' +
      '<div class="nc-status"><span class="nc-pill"><i data-lucide="usb"></i>USB <span class="nc-dim">· 1 ms</span></span>' +
      '<span class="nc-pill" data-saved><i data-lucide="check"></i>Enregistré</span>' +
      '<button class="nc-btn"><i data-lucide="settings"></i>Réglages</button></div></nav>';
  }

  function barcode(text, w) { // barres pseudo-aléatoires mais stables pour un texte donné
    var h = 0, x = 0, bars = '';
    for (var i = 0; i < text.length; i++) h = (h * 31 + text.charCodeAt(i)) >>> 0;
    while (x < w) { h = (h * 1103515245 + 12345) >>> 0; var bw = 1 + (h >>> 28) % 3; bars += '<rect x="' + x + '" width="' + bw + '" height="26"/>'; x += bw + 1 + (h >>> 24) % 3; }
    return '<div class="nc-barcode"><svg width="' + w + '" height="26" fill="currentColor" style="color:var(--ink-faint)">' + bars + '</svg><span>' + text + '</span></div>';
  }

  var STAR = '<svg viewBox="0 0 24 24" fill="currentColor"><path d="M12 0 L14 10 L24 12 L14 14 L12 24 L10 14 L0 12 L10 10 Z"/></svg>';

  function frame(reg, code) { // décor du cadre terminal : équerres, registre vertical, code-barres, une étoile
    return '<i class="nc-corner tl"></i><i class="nc-corner tr"></i><i class="nc-corner bl"></i><i class="nc-corner br"></i>' +
      '<span class="nc-reg vert">' + reg + '</span>' + barcode(code, 120) +
      '<span class="nc-star" style="right:30px;bottom:30px">' + STAR + '</span>';
  }

  /* ── Le pad en 3D, depuis le vrai modèle (case/model → STL → assets/Modele/nullpad.json) ──────────
     pad3d(THREE, geo, stage, o) — o: {legends[23], sel, lcd:{top,tag,bottom}, fill (part de la hauteur), cx (0–1), rot (°), callouts:{D1:[titre,texte],D2:…}, onPick}
     Renvoie {select, legends, lcd, hover, press, pick, resize, dispose, invalidate, frames} (ajouts nullctl, voir README-ecarts). Les légendes et l'écran sont des textures posées sur les surfaces : elles suivent la perspective. */
  function pad3d(THREE, geo, stage, o) {
    var W = stage.clientWidth, H = stage.clientHeight;
    var r = new THREE.WebGLRenderer({ antialias: true, alpha: true });
    r.setPixelRatio(Math.min(1.5, window.devicePixelRatio || 1)); r.setSize(W, H);
    r.toneMapping = THREE.ACESFilmicToneMapping; r.toneMappingExposure = 0.85; r.outputColorSpace = THREE.SRGBColorSpace;
    r.shadowMap.enabled = true; r.shadowMap.type = THREE.PCFSoftShadowMap;
    r.domElement.className = 'nc-canvas'; stage.appendChild(r.domElement);

    var scene = new THREE.Scene();
    scene.environment = studio(THREE, r);
    var M = {
      frosted: new THREE.MeshPhysicalMaterial({ color: 0xeeeef4, roughness: 0.45, transparent: true, opacity: 0.7, clearcoat: 0.5, clearcoatRoughness: 0.5 }),
      violet: new THREE.MeshPhysicalMaterial({ color: 0x6a3fe0, roughness: 0.32, transparent: true, opacity: 0.94, clearcoat: 0.3, clearcoatRoughness: 0.45, emissive: 0x3a1a8a, emissiveIntensity: 0.45, envMapIntensity: 0.3 }),
      white: new THREE.MeshPhysicalMaterial({ color: 0xe8e7ee, roughness: 0.6, clearcoat: 0.15 }),
      pcb: new THREE.MeshPhysicalMaterial({ color: 0x5a32c8, roughness: 0.45, clearcoat: 0.4, clearcoatRoughness: 0.4 }), // vernis épargne violet

      sw: new THREE.MeshStandardMaterial({ color: 0xd9d7e0, roughness: 0.5 }),
      metal: new THREE.MeshStandardMaterial({ color: 0xa9a7b0, roughness: 0.35, metalness: 0.8 }),
      module: new THREE.MeshStandardMaterial({ color: 0x24222a, roughness: 0.6 }),
      lcd: new THREE.MeshStandardMaterial({ color: 0xb8bbb3, roughness: 0.6 }),
      glass: new THREE.MeshPhysicalMaterial({ color: 0xffffff, roughness: 0.05, transparent: true, opacity: 0.1, clearcoat: 1, depthWrite: false })
    };
    function matFor(n) {
      if (VIOLET_KEYS[n] || /^D\d|^C/.test(n)) return M.violet;
      if (/^F\d/.test(n)) return M.white;
      if (/^SW/.test(n) || n === 'J1-fiche') return M.sw;
      if (/^EN|^USB/.test(n)) return M.metal;
      if (n === 'SuperMini') return M.module;
      if (n === 'PCB') return M.pcb;
      if (n === 'niceview') return M.lcd;
      if (n === 'vitre') return M.glass;
      return M.frosted;
    }
    var pad = new THREE.Group(), byName = {};
    var geos = geo.geos.map(function (g) {
      var q = new Uint16Array(Uint8Array.from(atob(g.pos), function (c) { return c.charCodeAt(0); }).buffer), f = new Float32Array(q.length);
      for (var i = 0; i < q.length; i++) f[i] = q[i] / 65535 * g.ext[i % 3];
      var bg = new THREE.BufferGeometry(); bg.setAttribute('position', new THREE.BufferAttribute(f, 3)); bg.computeVertexNormals(); bg.computeBoundingBox();
      return bg;
    });
    geo.meshes.forEach(function (m) {
      if (/^E\d/.test(m.name)) return; // pieds repliés dessous : invisibles
      var mat = matFor(m.name); if (/^F\d|^D\d/.test(m.name)) mat = mat.clone(); // matière propre : surbrillance par touche
      var mesh = new THREE.Mesh(geos[m.geo], mat); mesh.name = m.name; mesh.position.set(m.at[0], m.at[1], m.at[2]);
      mesh.castShadow = mat !== M.frosted && mat !== M.glass; mesh.receiveShadow = true;
      mesh.renderOrder = mat === M.glass ? 3 : mat === M.frosted ? 2 : 1;
      pad.add(mesh); byName[m.name] = mesh;
    });
    // Tout tourne autour du centre du pad (rotation à plat sur le bureau).
    var C = new THREE.Vector3(39.1, 80, 0), pivot = new THREE.Group();
    pad.position.set(-C.x, -C.y, 0); pivot.add(pad); pivot.rotation.z = THREE.MathUtils.degToRad(o.rot || 0); scene.add(pivot);
    // « NULL/PAD » tamponné sur la coque, sous M1–M3.
    var bc = document.createElement('canvas'); bc.width = 1024; bc.height = 96;
    var bx = bc.getContext('2d'); bx.fillStyle = '#8d8899'; bx.textAlign = 'center'; bx.textBaseline = 'middle';
    bx.font = '700 60px "IBM Plex Mono", monospace'; if ('letterSpacing' in bx) bx.letterSpacing = '24px'; bx.fillText('NULL/PAD', 512, 50);
    var bt = new THREE.CanvasTexture(bc); bt.colorSpace = THREE.SRGBColorSpace; bt.anisotropy = 8;
    var brand = new THREE.Mesh(new THREE.PlaneGeometry(34, 3.2), new THREE.MeshBasicMaterial({ map: bt, transparent: true, toneMapped: false }));
    brand.position.set(39.1, -0.3, 5.03); brand.renderOrder = 4; pad.add(brand);
    scene.updateMatrixWorld(true);

    scene.add(new THREE.HemisphereLight(0xffffff, 0x4a4360, 0.7));
    var key = new THREE.DirectionalLight(0xffffff, 2.0); key.position.set(-110, -170, 220); key.castShadow = true;
    key.shadow.mapSize.set(2048, 2048); key.shadow.bias = -0.0004; key.shadow.normalBias = 0.05;
    Object.assign(key.shadow.camera, { left: -110, right: 110, top: 110, bottom: -110, near: 1, far: 800 });
    scene.add(key);
    var rim = new THREE.DirectionalLight(0xb9a2ff, 1.6); rim.position.set(140, 180, 50); scene.add(rim);

    // Caméra : focale longue, inclinée de 24° vers l'utilisateur ; recul calculé pour que le pad remplisse `fill` de la hauteur.
    var fov = 16, tilt = THREE.MathUtils.degToRad(24), fill = o.fill || 0.9;
    var D = 168 / (fill * 2 * Math.tan(THREE.MathUtils.degToRad(fov / 2)));
    var cam = new THREE.PerspectiveCamera(fov, W / H, 10, 5000);
    function aim() { // nullctl : refait au redimensionnement
      var visW = 2 * D * Math.tan(THREE.MathUtils.degToRad(fov / 2)) * (W / H), dx = (0.5 - (o.cx == null ? 0.5 : o.cx)) * visW;
      cam.aspect = W / H; cam.updateProjectionMatrix();
      cam.position.set(dx, -D * Math.sin(tilt), D * Math.cos(tilt)); cam.up.set(0, 1, 0); cam.lookAt(dx, 3, 0);
      cam.updateMatrixWorld();
    }
    aim();

    // Faces du dessus : sommets les plus « hauts » selon la normale n → plan de cette face (centre, largeur, longueur).
    function topFace(mesh, n) {
      var p = mesh.geometry.attributes.position, best = -1e9, v = new THREE.Vector3(), pts = [];
      for (var i = 0; i < p.count; i++) { v.fromBufferAttribute(p, i); best = Math.max(best, v.dot(n)); }
      for (i = 0; i < p.count; i++) { v.fromBufferAttribute(p, i); if (v.dot(n) > best - 0.02) pts.push(v.clone()); }
      var b = new THREE.Box3().setFromPoints(pts), c = b.getCenter(new THREE.Vector3()), s = b.getSize(new THREE.Vector3());
      return { c: c.add(mesh.position), w: s.x, h: Math.hypot(s.y, s.z) };
    }
    function label(F, text) { // texture de légende posée 0,04 mm au-dessus du dessus de la keycap
      var mesh = byName[F]; if (!mesh) return;
      if (!mesh.userData.top) {
        var t = topFace(mesh, new THREE.Vector3(0, 0, 1)), cv = document.createElement('canvas'); cv.width = cv.height = 256;
        var tex = new THREE.CanvasTexture(cv); tex.colorSpace = THREE.SRGBColorSpace; tex.anisotropy = 8;
        var pl = new THREE.Mesh(new THREE.PlaneGeometry(t.w, t.h), new THREE.MeshBasicMaterial({ map: tex, transparent: true, toneMapped: false }));
        pl.position.copy(t.c).setZ(t.c.z + 0.04); pl.renderOrder = 4; pad.add(pl);
        mesh.userData.top = { cv: cv, tex: tex, t: t, pl: pl };
      }
      var u = mesh.userData.top, sig = String(text); // nullctl : texte (ou [l1, l2]) inchangé → pas de redessin
      if (u.text === sig) return; u.text = sig;
      var x = u.cv.getContext('2d');
      x.clearRect(0, 0, 256, 256); x.textAlign = 'center'; x.textBaseline = 'middle';
      if (Array.isArray(text) && text[1]) { // nullctl : légende sur deux lignes
        x.fillStyle = VIOLET_KEYS[F] ? '#f3efff' : '#2a2533'; x.font = '600 50px "IBM Plex Mono", monospace';
        x.fillText(text[0].toUpperCase(), 128, 104); x.fillText(text[1].toUpperCase(), 128, 160);
        u.tex.needsUpdate = true; return;
      }
      if (Array.isArray(text)) text = text[0] || '';
      var empty = !text, small = text && text.length > 4;
      x.fillStyle = VIOLET_KEYS[F] ? '#f3efff' : empty ? '#8f899a' : '#2a2533';
      x.font = (empty ? '400 104px ' : small ? '600 50px ' : '700 ' + (text.length > 2 ? 74 : 96) + 'px ') + '"IBM Plex Mono", monospace';
      x.fillText(empty ? '+' : small ? text.toUpperCase() : text, 128, 132);
      u.tex.needsUpdate = true;
    }
    // Écran nice!view : texture sur la face haute de l'écran (inclinée de 10°), sous la vitre.
    var nv = byName.niceview, lcdCv = document.createElement('canvas'); lcdCv.width = 160; lcdCv.height = 400;
    var a10 = THREE.MathUtils.degToRad(10), nN = new THREE.Vector3(0, -Math.sin(a10), Math.cos(a10)), nt = topFace(nv, nN);
    var lcdTex = new THREE.CanvasTexture(lcdCv); lcdTex.colorSpace = THREE.SRGBColorSpace; lcdTex.anisotropy = 8;
    var lcdPl = new THREE.Mesh(new THREE.PlaneGeometry(nt.w, nt.h), new THREE.MeshStandardMaterial({ map: lcdTex, roughness: 0.6 }));
    lcdPl.rotation.x = a10; lcdPl.position.copy(nt.c).addScaledVector(nN, 0.03); lcdPl.receiveShadow = true; pad.add(lcdPl);
    var lcdSig = null;
    function lcd(L) {
      var sig = [L.top, L.tag, L.bottom].join('\n'); if (sig === lcdSig) return; lcdSig = sig; // nullctl : pas de redessin inutile
      var x = lcdCv.getContext('2d'); x.fillStyle = '#b8bbb3'; x.fillRect(0, 0, 160, 400);
      x.fillStyle = '#1b1c19'; x.textAlign = 'center'; x.textBaseline = 'middle';
      x.font = '700 28px Silkscreen, monospace'; x.fillText(L.top || '', 80, 112);
      x.font = '700 62px Silkscreen, monospace'; x.fillText(L.tag || '', 80, 200);
      x.font = '700 28px Silkscreen, monospace'; x.fillText(L.bottom || '', 80, 288);
      lcdTex.needsUpdate = true;
    }

    // Sélection : contour violet net (arêtes) + légère lueur ; survol : lueur plus faible.
    var edgeMat = new THREE.LineBasicMaterial({ color: 0xa88cff }), edges = null, sel = null, hover = null;
    function glow(m, k) { if (m) { m.material.emissive.setHex(VIOLET_KEYS[m.name] || /^D/.test(m.name) ? 0x3a1a8a : 0x7b4dff); m.material.emissiveIntensity = VIOLET_KEYS[m.name] || /^D/.test(m.name) ? 0.45 + k : k; } }
    function select(name) {
      if (sel) glow(sel, 0); if (edges) { edges.parent.remove(edges); edges = null; }
      sel = byName[name] || null;
      if (sel) {
        glow(sel, 0.18);
        edges = new THREE.LineSegments(new THREE.EdgesGeometry(sel.geometry, 25), edgeMat); edges.position.copy(sel.position); edges.renderOrder = 5; pad.add(edges);
      }
      render(); anchor();
    }
    // Ancre HTML invisible sur la touche sélectionnée : sert au fil vers le panneau.
    var anc = document.createElement('div'); anc.className = 'nc-hit is-sel nc-anchor'; stage.appendChild(anc);
    function screenBox(mesh) {
      var b = mesh.geometry.boundingBox, xs = [], ys = [], v = new THREE.Vector3();
      [b.min.x, b.max.x].forEach(function (px) { [b.min.y, b.max.y].forEach(function (py) { [b.min.z, b.max.z].forEach(function (pz) {
        v.set(px, py, pz); mesh.localToWorld(v); v.project(cam); xs.push((v.x + 1) / 2 * W); ys.push((1 - v.y) / 2 * H);
      }); }); });
      return { x: Math.min.apply(0, xs), y: Math.min.apply(0, ys), w: Math.max.apply(0, xs) - Math.min.apply(0, xs), h: Math.max.apply(0, ys) - Math.min.apply(0, ys) };
    }
    function anchor() {
      if (!sel) { anc.style.display = 'none'; return; }
      var s = screenBox(sel); anc.style.cssText = 'left:' + s.x + 'px;top:' + s.y + 'px;width:' + s.w + 'px;height:' + s.h + 'px';
    }
    // nullctl : rendu à la demande. render() ne dessine plus : il demande UNE image (rAF unique en attente), rien si la fenêtre est cachée.
    var raf = 0, frames = 0, dead = false;
    function draw() { raf = 0; if (dead || document.hidden) return; r.render(scene, cam); frames++; }
    function render() { if (!raf && !dead) raf = requestAnimationFrame(draw); }
    function onVis() { if (!document.hidden) render(); }
    document.addEventListener('visibilitychange', onVis);

    // Clic et survol par lancer de rayon sur les keycaps et les molettes.
    var ray = new THREE.Raycaster(), ndc = new THREE.Vector2();
    var pickable = Object.keys(byName).filter(function (n) { return /^F\d|^D\d/.test(n); }).map(function (n) { return byName[n]; });
    function hit(e) {
      var b = r.domElement.getBoundingClientRect();
      ndc.set((e.clientX - b.left) / b.width * 2 - 1, -((e.clientY - b.top) / b.height) * 2 + 1); ray.setFromCamera(ndc, cam);
      var h = ray.intersectObjects(pickable, false)[0]; return h ? h.object : null;
    }
    r.domElement.addEventListener('pointermove', function (e) {
      var m = hit(e); r.domElement.style.cursor = m ? 'pointer' : ''; hoverOn(m);
    });
    r.domElement.addEventListener('pointerleave', function () { hoverOn(null); }); // nullctl
    function hoverOn(m) { if (m === hover) return; if (hover && hover !== sel) glow(hover, 0); hover = m; if (m && m !== sel) glow(m, 0.08); render(); }
    r.domElement.addEventListener('click', function (e) { var m = hit(e); if (m && o.onPick) o.onPick(m.name); });

    // Légendes des molettes, placées à côté de leur projection.
    if (o.callouts) [['D1', 'left'], ['D2', 'right']].forEach(function (d) {
      var c = o.callouts[d[0]]; if (!c) return;
      var s = screenBox(byName[d[0]]), el = document.createElement('div');
      el.className = 'nc-callout ' + d[1] + (o.sel === d[0] ? ' is-sel' : ''); el.dataset.key = d[0];
      el.innerHTML = '<span class="ln"></span><span class="tx"><b></b><span></span></span>';
      el.querySelector('b').textContent = c[0]; el.querySelector('.tx span').textContent = c[1];
      if (d[1] === 'left') el.style.cssText = 'right:' + (W - s.x + 4) + 'px;top:' + (s.y + s.h / 2 - 16) + 'px';
      else el.style.cssText = 'left:' + (s.x + s.w + 4) + 'px;top:' + (s.y + s.h / 2 - 16) + 'px';
      el.addEventListener('click', function () { if (o.onPick) o.onPick(d[0]); });
      stage.appendChild(el);
    });

    function legends(arr) { for (var i = 1; i <= 23; i++) label('F' + i, arr[i - 1] || ''); render(); }
    legends(o.legends || NUMPAD);
    lcd(o.lcd || { top: 'USB', tag: 'DEF', bottom: 'P1' });
    select(o.sel);
    // nullctl : appui (enfoncement de dz mm le long de l'axe de la touche, lueur violette), pick, redimensionnement, nettoyage.
    function press(name, down, dz) {
      var m = byName[name]; if (!m) return;
      if (m.userData.z0 == null) m.userData.z0 = m.position.z;
      var z = down ? -(dz == null ? 1.5 : dz) : 0, u = m.userData.top;
      m.position.z = m.userData.z0 + z; if (u) u.pl.position.z = u.t.c.z + 0.04 + z;
      if (edges && sel === m) edges.position.z = m.position.z;
      glow(m, down ? 0.35 : m === sel ? 0.18 : m === hover ? 0.08 : 0); render();
    }
    function pick(x, y) { var m = hit({ clientX: x, clientY: y }); return m ? m.name : null; }
    function resize(w, h) { if (!w || !h || (w === W && h === H)) return; W = w; H = h; r.setSize(W, H); aim(); anchor(); render(); }
    function dispose() {
      dead = true; if (raf) cancelAnimationFrame(raf); document.removeEventListener('visibilitychange', onVis);
      r.dispose(); r.domElement.remove(); anc.remove(); stage.querySelectorAll('.nc-callout').forEach(function (n) { n.remove(); });
    }
    return { select: select, legends: legends, lcd: function (L) { lcd(L); render(); },
      hover: function (n) { hoverOn(byName[n] || null); }, press: press, pick: pick, resize: resize, dispose: dispose,
      invalidate: render, frames: function () { return frames; } };
  }

  function studio(THREE, r) { // petit studio pour les reflets : une boîte sombre et trois panneaux lumineux
    var s = new THREE.Scene(), g = new THREE.BoxGeometry(1, 1, 1);
    var room = new THREE.Mesh(g, new THREE.MeshBasicMaterial({ color: 0x2a2730, side: THREE.BackSide })); room.scale.setScalar(40); s.add(room);
    [[0, 0, 18, 30, 30, 1, 3], [-15, -10, 6, 1, 14, 10, 2.2], [15, 8, 6, 1, 10, 12, 1.4]].forEach(function (a) {
      var m = new THREE.Mesh(g, new THREE.MeshBasicMaterial({ color: new THREE.Color(a[6], a[6], a[6]) })); m.position.set(a[0], a[1], a[2]); m.scale.set(a[3], a[4], a[5]); s.add(m);
    });
    return new THREE.PMREMGenerator(r).fromScene(s, 0.04).texture;
  }

  function select(stage, key) { // pour les légendes de molettes (le pad 3D gère ses touches lui-même)
    stage.querySelectorAll('.nc-callout').forEach(function (n) { n.classList.toggle('is-sel', n.dataset.key === key); });
  }

  /* Fil coudé de la touche sélectionnée au titre du panneau : relie visuellement les deux. */
  function wire(main) {
    var old = main.querySelector('.nc-wire'); if (old) old.remove();
    var hit = main.querySelector('.nc-callout.is-sel') || main.querySelector('.nc-hit.is-sel'), head = main.querySelector('.nc-panel-head');
    if (!hit || !head || hit.style.display === 'none') return;
    var m = main.getBoundingClientRect(), a = hit.getBoundingClientRect(), b = head.getBoundingClientRect();
    var x1 = a.right - m.left + 3, y1 = a.top + a.height / 2 - m.top, x2 = b.left - m.left, y2 = b.top + b.height / 2 - m.top;
    var xm = x2 - 28;
    var svg = '<svg class="nc-wire" width="' + m.width + '" height="' + m.height + '"><path d="M' + x1 + ' ' + y1 + ' H' + xm + ' V' + y2 + ' H' + x2 + '" fill="none" stroke="var(--violet-400)" stroke-width="1.5"/>' +
      '<rect x="' + (x1 - 3) + '" y="' + (y1 - 3) + '" width="6" height="6" fill="var(--violet-400)"/><rect x="' + (x2 - 4) + '" y="' + (y2 - 4) + '" width="8" height="8" fill="var(--violet-400)"/></svg>';
    main.insertAdjacentHTML('beforeend', svg);
  }

  /* Glitch d'événement (changement de profil, sauvegarde) : décalage RGB + balayage, 0,36 s, puis plus rien. */
  function glitch(el) {
    if (!document.getElementById('nc-rgb')) document.body.insertAdjacentHTML('beforeend',
      '<svg width="0" height="0" style="position:absolute"><filter id="nc-rgb"><feColorMatrix in="SourceGraphic" result="r" values="1 0 0 0 0  0 0 0 0 0  0 0 0 0 0  0 0 0 1 0"/>' +
      '<feOffset in="r" dx="-4" result="r2"/><feColorMatrix in="SourceGraphic" result="gb" values="0 0 0 0 0  0 1 0 0 0  0 0 1 0 0  0 0 0 1 0"/>' +
      '<feOffset in="gb" dx="3" result="gb2"/><feBlend in="r2" in2="gb2" mode="screen"/></filter></svg>');
    el.classList.remove('nc-glitch'); void el.offsetWidth; el.classList.add('nc-glitch');
    setTimeout(function () { el.classList.remove('nc-glitch'); }, 400);
  }

  function toast(app, html) {
    var t = document.createElement('div'); t.className = 'nc-toast'; t.innerHTML = html; app.appendChild(t);
    icons(); setTimeout(function () { t.remove(); }, 2600);
  }

  function icons() { if (window.lucide) window.lucide.createIcons(); }

  // Les polices doivent être chargées avant de dessiner les textures (légendes, écran).
  function fontsReady() {
    if (!document.fonts) return Promise.resolve();
    return Promise.all(['700 40px "IBM Plex Mono"', '600 40px "IBM Plex Mono"', '400 40px "IBM Plex Mono"', '700 20px Silkscreen'].map(function (f) { return document.fonts.load(f); }))
      .then(function () { return document.fonts.ready; });
  }

  window.NullCtl = { shell: shell, frame: frame, pad3d: pad3d, select: select, wire: wire, glitch: glitch, toast: toast, icons: icons, fontsReady: fontsReady, place: place, barcode: barcode, NUMPAD: NUMPAD, STAR: STAR };
})();
