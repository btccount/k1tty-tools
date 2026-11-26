"use strict";

(function () {
  const STORAGE_KEY = "k1tty_winter";
  const reducedMedia = window.matchMedia?.("(prefers-reduced-motion: reduce)");
  const isReduced = () => window.K1ttyAppearance?.isReducedMotion?.() ?? !!reducedMedia?.matches;
  let lastReduced = isReduced();

  let enabled = true;
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved === "0") enabled = false;
  } catch {}

  const glow = document.createElement("div");
  glow.id = "winterAtmosphere";
  glow.className = "winter-atmosphere";
  glow.setAttribute("aria-hidden", "true");

  const canvas = document.createElement("canvas");
  canvas.id = "winterCanvas";
  canvas.className = "winter-canvas";
  canvas.setAttribute("aria-hidden", "true");
  let ctx = null;

  function bakeSoftSprite(size, inner, mid) {
    const c = document.createElement("canvas");
    c.width = size;
    c.height = size;
    const g = c.getContext("2d");
    const r = size / 2;
    const grd = g.createRadialGradient(r, r, 0, r, r, r);
    grd.addColorStop(0, inner);
    grd.addColorStop(0.45, mid);
    grd.addColorStop(1, "rgba(210, 232, 255, 0)");
    g.fillStyle = grd;
    g.fillRect(0, 0, size, size);
    return c;
  }

  function bakeCrystalSprite(size) {
    const c = document.createElement("canvas");
    c.width = size;
    c.height = size;
    const g = c.getContext("2d");
    const half = size / 2;
    const arm = half * 0.78;
    const grd = g.createRadialGradient(half, half, 0, half, half, half);
    grd.addColorStop(0, "rgba(232, 245, 255, 0.36)");
    grd.addColorStop(0.5, "rgba(182, 218, 255, 0.08)");
    grd.addColorStop(1, "rgba(182, 218, 255, 0)");
    g.fillStyle = grd;
    g.fillRect(0, 0, size, size);

    g.save();
    g.translate(half, half);
    g.strokeStyle = "rgba(232, 245, 255, 0.92)";
    g.lineWidth = Math.max(1.1, size * 0.042);
    g.lineCap = "round";
    for (let i = 0; i < 6; i++) {
      g.beginPath();
      g.moveTo(0, 0);
      g.lineTo(0, -arm);
      const b1 = arm * 0.54;
      const b2 = arm * 0.76;
      const w1 = arm * 0.24;
      const w2 = arm * 0.16;
      g.moveTo(0, -b1);
      g.lineTo(-w1, -(b1 + w1 * 0.85));
      g.moveTo(0, -b1);
      g.lineTo(w1, -(b1 + w1 * 0.85));
      g.moveTo(0, -b2);
      g.lineTo(-w2, -(b2 + w2 * 0.85));
      g.moveTo(0, -b2);
      g.lineTo(w2, -(b2 + w2 * 0.85));
      g.stroke();
      g.rotate(Math.PI / 3);
    }
    g.restore();
    return c;
  }

  let SPR_MICRO, SPR_SOFT, SPR_BOKEH, SPR_CRYSTAL;
  function ensureResources() {
    if (ctx) return true;
    ctx = canvas.getContext("2d");
    if (!ctx) return false;
    SPR_MICRO = bakeSoftSprite(12, "rgba(238, 247, 255, 0.95)", "rgba(202, 226, 255, 0.35)");
    SPR_SOFT = bakeSoftSprite(20, "rgba(244, 250, 255, 0.95)", "rgba(196, 224, 255, 0.42)");
    SPR_BOKEH = bakeSoftSprite(28, "rgba(240, 248, 255, 0.78)", "rgba(176, 214, 255, 0.28)");
    SPR_CRYSTAL = bakeCrystalSprite(36);
    return true;
  }

  let w = 0;
  let h = 0;
  let dpr = 1;
  let flakes = [];
  let rafId = 0;
  let fadeTimer = 0;
  let fadeRafId = 0;
  let resizeRafId = 0;
  let scrollResumeTimer = 0;
  let scrollPaused = false;
  let windowBlurred = false;
  let prevTs = 0;
  let elapsed = 0;
  let pointerX = -9999;
  let pointerY = -9999;
  let pointerVx = 0;
  let lastPtrX = -9999;

  function initFlake(f, randomY) {
    const roll = Math.random();
    let layer = 0;
    let kind = "micro";
    let radius = 1.0;
    let speedY = 16;
    let driftAmp = 6;
    let alpha = 0.24;
    let rot = 0;
    let rotSpeed = 0;

    if (roll < 0.46) {
      layer = 0;
      kind = "micro";
      radius = 0.85 + Math.random() * 0.7;
      speedY = 11 + Math.random() * 11;
      driftAmp = 4 + Math.random() * 6;
      alpha = 0.16 + Math.random() * 0.2;
    } else if (roll < 0.85) {
      layer = 1;
      kind = "soft";
      radius = 1.6 + Math.random() * 1.35;
      speedY = 22 + Math.random() * 18;
      driftAmp = 9 + Math.random() * 12;
      alpha = 0.3 + Math.random() * 0.26;
    } else if (roll < 0.94) {
      layer = 2;
      kind = "bokeh";
      radius = 3.0 + Math.random() * 1.9;
      speedY = 34 + Math.random() * 20;
      driftAmp = 14 + Math.random() * 14;
      alpha = 0.24 + Math.random() * 0.22;
    } else {
      layer = 2;
      kind = "crystal";
      radius = 5.2 + Math.random() * 3.2;
      speedY = 18 + Math.random() * 14;
      driftAmp = 12 + Math.random() * 12;
      alpha = 0.22 + Math.random() * 0.2;
      rot = Math.random() * Math.PI * 2;
      rotSpeed = (Math.random() < 0.5 ? -1 : 1) * (0.14 + Math.random() * 0.26);
    }

    f.x = Math.random() * (w + 60) - 30;
    f.y = randomY ? Math.random() * h : -(radius * 2 + Math.random() * 48);
    f.layer = layer;
    f.kind = kind;
    f.r = radius;
    f.vy = speedY;
    f.vx = 0;
    f.amp = driftAmp;
    f.freq = 0.35 + Math.random() * 0.65;
    f.phase = Math.random() * Math.PI * 2;
    f.alpha = alpha;
    f.twinkle = 0.4 + Math.random() * 0.9;
    f.rot = rot;
    f.rotSpeed = rotSpeed;
    return f;
  }

  function targetCount() {
    const area = Math.max(1, w * h);
    return Math.max(52, Math.min(120, Math.round(area / 17500)));
  }

  function syncSize() {
    if (!ensureResources()) return;
    const nextW = Math.max(1, window.innerWidth);
    const nextH = Math.max(1, window.innerHeight);
    const nextDpr = Math.min(2, window.devicePixelRatio || 1);
    if (w === nextW && h === nextH && dpr === nextDpr && canvas.width > 1) return;
    w = nextW;
    h = nextH;
    dpr = nextDpr;
    canvas.width = Math.round(w * dpr);
    canvas.height = Math.round(h * dpr);
    canvas.style.width = w + "px";
    canvas.style.height = h + "px";
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);

    const want = targetCount();
    if (flakes.length === 0) {
      for (let i = 0; i < want; i++) flakes.push(initFlake({}, true));
    } else if (flakes.length < want) {
      for (let i = flakes.length; i < want; i++) flakes.push(initFlake({}, true));
    } else if (flakes.length > want) {
      flakes.length = want;
    }
  }

  function renderFrame(dt) {
    const animate = !isReduced();
    ctx.clearRect(0, 0, w, h);
    const wind = Math.sin(elapsed * 0.24) * 13 + Math.cos(elapsed * 0.09) * 7;
    pointerVx *= Math.pow(0.08, dt);
    const drag = Math.pow(0.18, dt);

    for (let i = 0; i < flakes.length; i++) {
      const f = flakes[i];
      if (animate) {
        const sway = Math.sin(elapsed * f.freq + f.phase) * f.amp;
        const layerWind = wind * (0.45 + f.layer * 0.35);

        if (f.layer >= 1 && pointerX > -1000) {
          const dx = f.x - pointerX;
          const dy = f.y - pointerY;
          const distSq = dx * dx + dy * dy;
          if (distSq < 15625 && distSq > 4) {
            const dist = Math.sqrt(distSq);
            const falloff = 1 - dist / 125;
            const push = falloff * falloff * (18 + f.layer * 14);
            f.vx += ((dx / dist) * push + pointerVx * 0.14 * falloff) * dt;
          }
        }

        f.vx *= drag;
        f.x += (layerWind + sway + f.vx) * dt;
        f.y += f.vy * dt;
        f.rot += f.rotSpeed * dt;

        if (f.y > h + f.r * 3) {
          initFlake(f, false);
          continue;
        }
        if (f.x < -36) f.x = w + 30;
        else if (f.x > w + 36) f.x = -30;
      }

      const shimmer = 0.86 + 0.14 * Math.sin(elapsed * f.twinkle + f.phase);
      ctx.globalAlpha = f.alpha * shimmer;
      const d = f.r * 2;
      if (f.kind === "crystal") {
        ctx.save();
        ctx.translate(f.x, f.y);
        ctx.rotate(f.rot);
        ctx.drawImage(SPR_CRYSTAL, -f.r, -f.r, d, d);
        ctx.restore();
      } else if (f.kind === "bokeh") {
        ctx.drawImage(SPR_BOKEH, f.x - f.r, f.y - f.r, d, d);
      } else if (f.kind === "soft") {
        ctx.drawImage(SPR_SOFT, f.x - f.r, f.y - f.r, d, d);
      } else {
        ctx.drawImage(SPR_MICRO, f.x - f.r, f.y - f.r, d, d);
      }
    }
    ctx.globalAlpha = 1;
  }

  function tick(ts) {
    rafId = 0;
    if (!enabled || document.hidden || isReduced() || windowBlurred || scrollPaused || !ctx) return;
    if (prevTs && (ts - prevTs) < 32) {
      rafId = requestAnimationFrame(tick);
      return;
    }
    const dt = prevTs ? Math.min(0.06, Math.max(0.001, (ts - prevTs) / 1000)) : 0.033;
    prevTs = ts;
    elapsed += dt;
    renderFrame(dt);
    rafId = requestAnimationFrame(tick);
  }

  function startLoop() {
    if (!enabled || document.hidden || isReduced() || windowBlurred || scrollPaused || !ctx) return;
    if (rafId) return;
    prevTs = 0;
    rafId = requestAnimationFrame(tick);
  }

  function stopLoop() {
    if (rafId) {
      cancelAnimationFrame(rafId);
      rafId = 0;
    }
    if (resizeRafId) { cancelAnimationFrame(resizeRafId); resizeRafId = 0; }
    if (scrollResumeTimer) { clearTimeout(scrollResumeTimer); scrollResumeTimer = 0; }
    scrollPaused = false;
    prevTs = 0;
  }

  function applyState(next, persist) {
    if (persist && !!next === enabled) return;
    enabled = !!next;
    if (persist) {
      try {
        localStorage.setItem(STORAGE_KEY, enabled ? "1" : "0");
      } catch {}
    }
    if (fadeTimer) {
      clearTimeout(fadeTimer);
      fadeTimer = 0;
    }
    if (fadeRafId) { cancelAnimationFrame(fadeRafId); fadeRafId = 0; }
    document.body.classList.toggle("winter-theme", enabled);
    if (enabled && !isReduced() && !document.hidden) {
      syncSize();
      canvas.classList.remove("hidden");
      fadeRafId = requestAnimationFrame(() => {
        fadeRafId = 0;
        if (enabled && !document.hidden && !isReduced()) canvas.classList.add("visible");
      });
      if (!windowBlurred) startLoop();
    } else {
      stopLoop();
      canvas.classList.remove("visible");
      if (!ctx) { canvas.classList.add("hidden"); return; }
      fadeTimer = setTimeout(() => {
        fadeTimer = 0;
        if (!enabled || isReduced() || document.hidden) {
          canvas.classList.add("hidden");
          canvas.width = canvas.height = 1;
        }
      }, 180);
    }
    window.K1ttyCat?.refresh?.();
  }

  window.addEventListener("pointermove", (ev) => {
    if (!enabled || document.hidden || isReduced() || windowBlurred) return;
    if (lastPtrX > -1000) {
      pointerVx = Math.max(-180, Math.min(180, (ev.clientX - lastPtrX) * 6));
    }
    lastPtrX = ev.clientX;
    pointerX = ev.clientX;
    pointerY = ev.clientY;
  }, { passive: true });

  window.addEventListener("pointerleave", () => {
    pointerX = -9999;
    pointerY = -9999;
    lastPtrX = -9999;
  }, { passive: true });

  document.addEventListener("scroll", (ev) => {
    if (!enabled || document.hidden || isReduced() || windowBlurred) return;
    if (ev.target && ev.target.id === "feedScroll") {
      scrollPaused = true;
      if (rafId) {
        cancelAnimationFrame(rafId);
        rafId = 0;
      }
      prevTs = 0;
      if (scrollResumeTimer) clearTimeout(scrollResumeTimer);
      scrollResumeTimer = setTimeout(() => {
        scrollResumeTimer = 0;
        scrollPaused = false;
        startLoop();
      }, 180);
    }
  }, { capture: true, passive: true });

  window.addEventListener("resize", () => {
    if (!enabled || document.hidden || isReduced() || resizeRafId) return;
    resizeRafId = requestAnimationFrame(() => {
      resizeRafId = 0;
      if (enabled && !document.hidden && !isReduced()) syncSize();
    });
  }, { passive: true });

  window.addEventListener("blur", () => {
    windowBlurred = true;
    stopLoop();
  });

  window.addEventListener("focus", () => {
    if (!windowBlurred) return;
    windowBlurred = false;
    if (enabled && !document.hidden && !isReduced()) {
      syncSize();
      startLoop();
    }
  });

  document.addEventListener("visibilitychange", () => {
    if (document.hidden) {
      stopLoop();
      if (fadeRafId) { cancelAnimationFrame(fadeRafId); fadeRafId = 0; }
    } else if (enabled && !isReduced() && !windowBlurred) {
      syncSize();
      canvas.classList.remove("hidden");
      if (!isReduced()) canvas.classList.add("visible");
      startLoop();
    }
  });

  function syncMotion() {
    const reduced = isReduced();
    if (lastReduced === reduced) return;
    lastReduced = reduced;
    if (reduced) {
      stopLoop();
      if (fadeRafId) { cancelAnimationFrame(fadeRafId); fadeRafId = 0; }
      canvas.classList.remove("visible");
    } else if (enabled && !document.hidden) {
      applyState(enabled, false);
    }
  }
  window.addEventListener("k1tty:appearancechange", syncMotion);
  reducedMedia?.addEventListener?.("change", syncMotion);

  function mount() {
    document.body.appendChild(glow);
    document.body.appendChild(canvas);
    applyState(enabled, false);
  }

  window.K1ttyWinter = {
    setEnabled: (v) => applyState(v, true),
    isEnabled: () => enabled,
    toggle: () => {
      applyState(!enabled, true);
      return enabled;
    },
  };

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", mount, { once: true });
  } else {
    mount();
  }
})();
