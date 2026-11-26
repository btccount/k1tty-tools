"use strict";

(function () {
  const STORAGE_KEY = "k1tty_cat";
  const PAL = {
    K: "#141417",
    W: "#f2f0ea",
    S: "#cbc5ba",
    P: "#f09ab0",
    A: "#5b9cf6",
    Z: "#a1a1a6",
    B: "#6caeff",
    I: "#eaf4ff",
  };

  const RAW = {
    sit1: [
      "................",
      "....KK....KK....",
      "...KWKK..KKWK...",
      "...KPKKKKKKPK...",
      "...KWWWWWWWWK...",
      "...KWKWWWWKWK...",
      "...KWWWPWWWWK...",
      "....KWWWWWWK..K.",
      "....KWWWWWWK.KWK",
      "...KWWSSSSWWKKWK",
      "...KWWSSSSWWKWK.",
      "...KWWWWWWWWKK..",
      "...KWWWWWWWWK...",
      "....KKWKKWKK....",
      ".....KK..KK.....",
      "................",
    ],
    sit2: [
      "................",
      "....KK....KK....",
      "...KWKK..KKWK...",
      "...KPKKKKKKPK...",
      "...KWWWWWWWWK...",
      "...KSSWWWWSSK...",
      "...KWWWPWWWWK...",
      "....KWWWWWWK...K",
      "....KWWWWWWK..KK",
      "...KWWSSSSWWKKWK",
      "...KWWSSSSWWKK..",
      "...KWWWWWWWWK...",
      "...KWWWWWWWWK...",
      "....KKWKKWKK....",
      ".....KK..KK.....",
      "................",
    ],
    groom1: [
      "................",
      "....KK....KK....",
      "...KWKK..KKWK...",
      "...KPKKKKKKPK...",
      "...KWWWWWWWWK...",
      "...KSSWWWWSSK...",
      "...KWWWPWWKWK...",
      "....KWWWWKKWK.K.",
      "....KWWWWWWK.KWK",
      "...KWWSSSSWWKKWK",
      "...KWWSSSSWWKWK.",
      "...KWWWWWWWWKK..",
      "...KWWWWWWWWK...",
      "....KKWKKWKK....",
      ".....KK..KK.....",
      "................",
    ],
    groom2: [
      "................",
      "....KK....KK....",
      "...KWKK..KKWK...",
      "...KPKKKKKKPK...",
      "...KWWWWWWWWK...",
      "...KSSWWWWKWK...",
      "...KWWWPWKKWK...",
      "....KWWWWWWK..K.",
      "....KWWWWWWK.KWK",
      "...KWWSSSSWWKKWK",
      "...KWWSSSSWWKWK.",
      "...KWWWWWWWWKK..",
      "...KWWWWWWWWK...",
      "....KKWKKWKK....",
      ".....KK..KK.....",
      "................",
    ],
    alert: [
      ".......AA.......",
      "....KK.AA.KK....",
      "...KWKK..KKWK...",
      "...KPKKAAKKPK...",
      "...KWWWWWWWWK...",
      "...KWKWWWWKWK...",
      "...KWWWPWWWWK...",
      "....KWWWWWWK..K.",
      "....KWWWWWWK.KWK",
      "...KWWSSSSWWKKWK",
      "...KWWSSSSWWKWK.",
      "...KWWWWWWWWKK..",
      "...KWWWWWWWWK...",
      "....KKWKKWKK....",
      ".....KK..KK.....",
      "................",
    ],
    yawn: [
      "................",
      "....KK....KK....",
      "...KWKK..KKWK...",
      "...KPKKKKKKPK...",
      "...KWWWWWWWWK...",
      "...KSSWWWWSSK...",
      "...KWWPPWWWWK...",
      "....KWPPWWWK..K.",
      "....KWWWWWWK.KWK",
      "...KWWSSSSWWKKWK",
      "...KWWSSSSWWKWK.",
      "...KWWWWWWWWKK..",
      "...KWWWWWWWWK...",
      "....KKWKKWKK....",
      ".....KK..KK.....",
      "................",
    ],
    sleep1: [
      "...........ZZZ..",
      "............Z...",
      "...........ZZZ..",
      "................",
      "................",
      "....KK...KK.....",
      "...KWKKKKWK.....",
      "..KPKWWWWKPKKK..",
      "..KWWWWWWWWWWWK.",
      "..KWSSWWSSWWWWWK",
      ".KWWWWWWWWSSWWWK",
      ".KWWWWWSSWWWWWWK",
      "..KKKKKKKKKKKKK.",
      "................",
      "................",
      "................",
    ],
    sleep2: [
      "............ZZZ.",
      ".............Z..",
      "............ZZZ.",
      "................",
      "................",
      "................",
      "....KK...KK.....",
      "...KWKKKKWK.....",
      "..KPKWWWWKPKKK..",
      "..KWWWWWWWWWWWK.",
      "..KWSSWWSSWWWWWK",
      ".KWWWWWWWSSWWWWK",
      "..KKKKKKKKKKKKK.",
      "................",
      "................",
      "................",
    ],
    walkR1: [
      "................",
      "................",
      ".........KK..KK.",
      "........KWKKKKWK",
      "..KK....KPKWWKPK",
      ".KWWK...KWWWWWWK",
      ".KWWK..KWWWKWWWK",
      "..KWKKKKWWWWWWPK",
      "...KWWWWWWWWWWK.",
      "...KWWSSSSWWWWK.",
      "...KWWWWWWWWWK..",
      "..KKWWK...KWWKK.",
      "..KWWK.....KWWK.",
      "...KK.......KK..",
      "................",
      "................",
    ],
    walkR2: [
      "................",
      ".........KK..KK.",
      ".KK.....KWKKKKWK",
      "KWWK....KPKWWKPK",
      ".KWK....KWWWWWWK",
      "..KWK..KWWWKWWWK",
      "..KWKKKKWWWWWWPK",
      "...KWWWWWWWWWWK.",
      "...KWWSSSSWWWWK.",
      "...KWWWWWWWWWK..",
      "....KWWK.KWWK...",
      "....KWWK.KWWK...",
      ".....KK...KK....",
      "................",
      "................",
      "................",
    ],
  };

  function flipRows(rows) {
    return rows.map((r) => r.split("").reverse().join(""));
  }

  RAW.walkL1 = flipRows(RAW.walkR1);
  RAW.walkL2 = flipRows(RAW.walkR2);

  function bakeSprite(rows) {
    const c = document.createElement("canvas");
    c.width = 16;
    c.height = 16;
    const g = c.getContext("2d");
    for (let y = 0; y < rows.length; y++) {
      const row = rows[y];
      for (let x = 0; x < row.length; x++) {
        const ch = row[x];
        const col = PAL[ch];
        if (!col) continue;
        g.fillStyle = col;
        g.fillRect(x, y, 1, 1);
      }
    }
    return c;
  }

  function applyWinterScarf(name, rows) {
    const out = rows.slice();
    const paint = (y, x, str) => {
      if (y < 0 || y >= out.length) return;
      const r = out[y];
      out[y] = r.slice(0, x) + str + r.slice(x + str.length);
    };
    if (name === "sit1" || name === "sit2" || name === "groom1" || name === "groom2" || name === "alert" || name === "yawn") {
      paint(7, 5, "BIBBIB");
      paint(8, 9, "BI");
      paint(9, 10, "B");
    } else if (name === "walkR1") {
      paint(8, 9, "BIBB");
      paint(9, 8, "BI");
    } else if (name === "walkR2") {
      paint(7, 9, "BIBB");
      paint(8, 8, "BI");
    } else if (name === "walkL1") {
      paint(8, 3, "BBIB");
      paint(9, 6, "IB");
    } else if (name === "walkL2") {
      paint(7, 3, "BBIB");
      paint(8, 6, "IB");
    } else if (name === "sleep1") {
      paint(10, 4, "BIBB");
    } else if (name === "sleep2") {
      paint(11, 4, "BIBB");
    }
    return out;
  }

  const SPRITES = {};
  const WINTER_SPRITES = {};

  const SIZE = 32;
  const STOP_DIST = 54;
  const WAKE_DIST = 82;
  const TOO_CLOSE = 28;
  const WALK_SPEED = 78;

  const canvas = document.createElement("canvas");
  canvas.id = "pixelCat";
  canvas.className = "pixel-cat";
  canvas.width = 16;
  canvas.height = 16;
  canvas.setAttribute("aria-hidden", "true");
  let ctx = null;

  let enabled = true;
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved === "0") enabled = false;
  } catch {}

  const reducedMedia = window.matchMedia?.("(prefers-reduced-motion: reduce)");
  const isReduced = () => window.K1ttyAppearance?.isReducedMotion?.() ?? !!reducedMedia?.matches;
  let lastReduced = isReduced();
  let viewW = Math.max(1, window.innerWidth || 1280);
  let viewH = Math.max(1, window.innerHeight || 840);
  let windowBlurred = false;
  let hasPointer = false;
  let mx = viewW * 0.5;
  let my = viewH * 0.5;
  let cx = viewW - 64;
  let cy = viewH - 64;
  let facing = "L";
  let mode = "idle";
  let idleTime = 0;
  let alertTime = 0;
  let animTime = 0;
  let blinkPhase = 0;
  let nextBlink = 3.2;
  let lastFrame = "";
  let rafId = 0;
  let idleTimer = 0;
  let prevTs = 0;
  let lastX = null;
  let lastY = null;

  let lastWinter = false;

  function syncViewport() {
    viewW = Math.max(1, window.innerWidth || 1280);
    viewH = Math.max(1, window.innerHeight || 840);
  }

  function draw(name) {
    if (!enabled || document.hidden || windowBlurred) return;
    const isWinter = document.body.classList.contains("winter-theme");
    if (name === lastFrame && isWinter === lastWinter) return;
    if (!ctx) {
      ctx = canvas.getContext("2d");
      if (!ctx) return;
      ctx.imageSmoothingEnabled = false;
    }
    lastFrame = name;
    lastWinter = isWinter;
    ctx.clearRect(0, 0, 16, 16);
    const sprites = isWinter ? WINTER_SPRITES : SPRITES;
    const sp = sprites[name] || (RAW[name] && (sprites[name] = bakeSprite(isWinter ? applyWinterScarf(name, RAW[name]) : RAW[name])));
    if (sp) ctx.drawImage(sp, 0, 0);
  }

  function refresh() {
    if (!enabled || !lastFrame) return;
    const prev = lastFrame;
    lastFrame = "";
    draw(prev);
  }

  function place() {
    const px = Math.round(cx - SIZE / 2);
    const py = Math.round(cy - SIZE / 2);
    if (px === lastX && py === lastY) return;
    lastX = px;
    lastY = py;
    canvas.style.transform = `translate3d(${px}px, ${py}px, 0)`;
  }

  function clampPos() {
    const pad = 20;
    cx = Math.max(pad, Math.min(viewW - pad, cx));
    cy = Math.max(pad, Math.min(viewH - pad, cy));
  }

  function step(ts) {
    rafId = 0;
    if (!enabled || document.hidden || windowBlurred || isReduced()) return;
    const dt = prevTs ? Math.min(0.3, Math.max(0.001, (ts - prevTs) / 1000)) : 0.016;
    const movementDt = Math.min(0.06, dt);
    prevTs = ts;
    animTime += dt;

    let moving = false;
    if (hasPointer) {
      const dx = mx - cx;
      const dy = my - cy;
      const dist = Math.hypot(dx, dy);

      if (mode === "alert") {
        moving = true;
        alertTime -= dt;
        draw("alert");
        if (alertTime <= 0) {
          mode = "walk";
        }
      } else if (mode === "walk") {
        if (dist <= STOP_DIST) {
          mode = "idle";
          idleTime = 0;
          nextBlink = 2.4 + Math.random() * 2.8;
        } else {
          moving = true;
          const stepDist = Math.min(WALK_SPEED * movementDt, dist - STOP_DIST + 1);
          const nx = dx / (dist || 1);
          const ny = dy / (dist || 1);
          cx += nx * stepDist;
          cy += ny * stepDist;
          clampPos();
          if (Math.abs(dx) > 4) facing = dx >= 0 ? "R" : "L";
          const frameIdx = Math.floor(animTime * 5.2) % 2;
          draw((facing === "R" ? "walkR" : "walkL") + (frameIdx + 1));
        }
      } else {
        if (dist > WAKE_DIST) {
          moving = true;
          if (idleTime > 9) {
            mode = "alert";
            alertTime = 0.28;
            draw("alert");
          } else {
            mode = "walk";
          }
        } else if (dist < TOO_CLOSE && dist > 0.5) {
          moving = true;
          const push = Math.min(WALK_SPEED * 0.75 * movementDt, STOP_DIST - dist);
          cx -= (dx / dist) * push;
          cy -= (dy / dist) * push;
          clampPos();
          facing = dx <= 0 ? "R" : "L";
          const frameIdx = Math.floor(animTime * 5.2) % 2;
          draw((facing === "R" ? "walkR" : "walkL") + (frameIdx + 1));
          idleTime = 0;
        } else {
          idleTime += dt;
          if (idleTime > 14) {
            draw(Math.floor(animTime * 1.1) % 2 === 0 ? "sleep1" : "sleep2");
          } else if (idleTime > 12.6) {
            draw("yawn");
          } else if (idleTime > 5.5 && idleTime < 8.0) {
            draw(Math.floor(animTime * 4) % 2 === 0 ? "groom1" : "groom2");
          } else {
            blinkPhase += dt;
            if (blinkPhase >= nextBlink) {
              draw("sit2");
              if (blinkPhase >= nextBlink + 0.18) {
                blinkPhase = 0;
                nextBlink = 2.8 + Math.random() * 3.4;
              }
            } else {
              draw("sit1");
            }
          }
        }
      }
    } else {
      draw("sit1");
    }

    place();
    if (moving) {
      rafId = requestAnimationFrame(step);
    } else if (hasPointer) {
       
       
      idleTimer = setTimeout(() => {
        idleTimer = 0;
        if (enabled && !document.hidden && !windowBlurred && !isReduced()) rafId = requestAnimationFrame(step);
      }, idleTime > 14 ? 250 : 100);
    }
  }

  function startLoop() {
    if (!enabled || document.hidden || windowBlurred || isReduced() || !hasPointer || rafId) return;
    if (idleTimer) { clearTimeout(idleTimer); idleTimer = 0; }
    rafId = requestAnimationFrame(step);
  }

  function stopLoop() {
    if (idleTimer) { clearTimeout(idleTimer); idleTimer = 0; }
    if (rafId) {
      cancelAnimationFrame(rafId);
      rafId = 0;
    }
    prevTs = 0;
  }

  function setEnabled(next) {
    if (!!next === enabled) return;
    enabled = !!next;
    try {
      localStorage.setItem(STORAGE_KEY, enabled ? "1" : "0");
    } catch {}
    canvas.classList.toggle("hidden", !enabled);
    if (enabled) {
      syncViewport();
      clampPos();
      draw("sit1");
      place();
      startLoop();
    } else {
      stopLoop();
    }
  }

  window.addEventListener("pointermove", (ev) => {
    if (!enabled || document.hidden || windowBlurred || isReduced()) return;
    mx = ev.clientX;
    my = ev.clientY;
    if (!hasPointer) {
      hasPointer = true;
      cx = Math.max(24, Math.min(viewW - 24, mx + 64));
      cy = Math.max(24, Math.min(viewH - 24, my + 24));
      place();
      startLoop();
      return;
    }
    if (mode === "idle") {
      const dx = mx - cx;
      const dy = my - cy;
      const distSq = dx * dx + dy * dy;
      if (distSq <= WAKE_DIST * WAKE_DIST && distSq >= TOO_CLOSE * TOO_CLOSE) return;
    }
    startLoop();
  }, { passive: true });

  window.addEventListener("resize", () => {
    if (!enabled || document.hidden) return;
    syncViewport();
    clampPos();
    place();
  }, { passive: true });

  window.addEventListener("blur", () => {
    windowBlurred = true;
    stopLoop();
  });

  window.addEventListener("focus", () => {
    if (!windowBlurred) return;
    windowBlurred = false;
    if (enabled && !document.hidden && !isReduced()) startLoop();
  });

  document.addEventListener("visibilitychange", () => {
    if (document.hidden) stopLoop();
    else if (enabled && !windowBlurred) startLoop();
  });

  function syncMotion() {
    const reduced = isReduced();
    if (lastReduced === reduced) return;
    lastReduced = reduced;
    stopLoop();
    if (!enabled) return;
    draw("sit1");
    if (!reduced && !windowBlurred) startLoop();
  }
  window.addEventListener("k1tty:appearancechange", syncMotion);
  reducedMedia?.addEventListener?.("change", syncMotion);

  function mount() {
    document.body.appendChild(canvas);
    syncViewport();
    draw("sit1");
    clampPos();
    place();
    canvas.classList.toggle("hidden", !enabled);
    if (enabled && !windowBlurred) startLoop();
  }

  window.K1ttyCat = {
    setEnabled,
    isEnabled: () => enabled,
    refresh,
  };

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", mount, { once: true });
  } else {
    mount();
  }
})();
