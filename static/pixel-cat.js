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
  for (const [k, rows] of Object.entries(RAW)) {
    SPRITES[k] = bakeSprite(rows);
    WINTER_SPRITES[k] = bakeSprite(applyWinterScarf(k, rows));
  }

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
  const ctx = canvas.getContext("2d");
  ctx.imageSmoothingEnabled = false;

  let enabled = true;
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved === "0") enabled = false;
  } catch {}

  const reduced = window.matchMedia?.("(prefers-reduced-motion: reduce)")?.matches;
  let hasPointer = false;
  let mx = window.innerWidth * 0.5;
  let my = window.innerHeight * 0.5;
  let cx = window.innerWidth - 64;
  let cy = window.innerHeight - 64;
  let facing = "L";
  let mode = "idle";
  let idleTime = 0;
  let alertTime = 0;
  let animTime = 0;
  let blinkPhase = 0;
  let nextBlink = 3.2;
  let lastFrame = "";
  let rafId = 0;
  let prevTs = 0;

  let lastWinter = false;

  function draw(name) {
    const isWinter = document.body.classList.contains("winter-theme");
    if (name === lastFrame && isWinter === lastWinter) return;
    lastFrame = name;
    lastWinter = isWinter;
    ctx.clearRect(0, 0, 16, 16);
    const sp = (isWinter ? WINTER_SPRITES : SPRITES)[name];
    if (sp) ctx.drawImage(sp, 0, 0);
  }

  function refresh() {
    if (!lastFrame) return;
    const prev = lastFrame;
    lastFrame = "";
    draw(prev);
  }

  function place() {
    const px = Math.round(cx - SIZE / 2);
    const py = Math.round(cy - SIZE / 2);
    canvas.style.transform = `translate3d(${px}px, ${py}px, 0)`;
  }

  function clampPos() {
    const pad = 20;
    cx = Math.max(pad, Math.min(window.innerWidth - pad, cx));
    cy = Math.max(pad, Math.min(window.innerHeight - pad, cy));
  }

  function step(ts) {
    rafId = 0;
    if (!enabled || document.hidden) return;
    const dt = prevTs ? Math.min(0.06, Math.max(0.001, (ts - prevTs) / 1000)) : 0.016;
    prevTs = ts;
    animTime += dt;

    if (hasPointer && !reduced) {
      const dx = mx - cx;
      const dy = my - cy;
      const dist = Math.hypot(dx, dy);

      if (mode === "alert") {
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
          const stepDist = Math.min(WALK_SPEED * dt, dist - STOP_DIST + 1);
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
          if (idleTime > 9) {
            mode = "alert";
            alertTime = 0.28;
            draw("alert");
          } else {
            mode = "walk";
          }
        } else if (dist < TOO_CLOSE && dist > 0.5) {
          const push = Math.min(WALK_SPEED * 0.75 * dt, STOP_DIST - dist);
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
    rafId = requestAnimationFrame(step);
  }

  function startLoop() {
    if (!enabled || rafId) return;
    prevTs = 0;
    rafId = requestAnimationFrame(step);
  }

  function stopLoop() {
    if (rafId) {
      cancelAnimationFrame(rafId);
      rafId = 0;
    }
  }

  function setEnabled(next) {
    enabled = !!next;
    try {
      localStorage.setItem(STORAGE_KEY, enabled ? "1" : "0");
    } catch {}
    canvas.classList.toggle("hidden", !enabled);
    if (enabled) {
      clampPos();
      draw("sit1");
      place();
      startLoop();
    } else {
      stopLoop();
    }
  }

  window.addEventListener("pointermove", (ev) => {
    mx = ev.clientX;
    my = ev.clientY;
    if (!hasPointer) {
      hasPointer = true;
      cx = Math.max(24, Math.min(window.innerWidth - 24, mx + 64));
      cy = Math.max(24, Math.min(window.innerHeight - 24, my + 24));
      place();
    }
  }, { passive: true });

  window.addEventListener("resize", () => {
    clampPos();
    place();
  }, { passive: true });

  document.addEventListener("visibilitychange", () => {
    if (document.hidden) stopLoop();
    else if (enabled) startLoop();
  });

  function mount() {
    document.body.appendChild(canvas);
    draw("sit1");
    clampPos();
    place();
    canvas.classList.toggle("hidden", !enabled);
    if (enabled) startLoop();
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
