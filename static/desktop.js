"use strict";

(() => {
   
   
  const agent = typeof navigator === "undefined" ? "" : navigator.userAgent;
  if (document.documentElement) {
    document.documentElement.dataset.k1ttyPlatform = /Macintosh|Mac OS X/.test(agent)
      ? "macos" : /Linux/.test(agent) ? "linux" : "windows";
  }

   
   
  const isTextInput = (target) => !!target?.closest?.(
    'input:not([type="button"]):not([type="checkbox"]):not([type="radio"]):not([type="range"]), textarea, [contenteditable]:not([contenteditable="false"])'
  );
  document.addEventListener("contextmenu", (event) => {
     
    event.preventDefault();
  }, true);

  document.addEventListener("keydown", (event) => {
    const key = event.key || "";
    const code = event.code || (key.length === 1 ? `Key${key.toUpperCase()}` : key);
    const modifier = event.ctrlKey || event.metaKey;
    const editing = isTextInput(event.target);
    const inspector = key === "F12" || (modifier && (event.shiftKey || event.altKey) && ["KeyI", "KeyJ", "KeyC"].includes(code));
    const browserCommand = modifier && ["KeyR", "KeyS", "KeyP", "KeyU", "KeyL", "KeyO", "KeyJ", "KeyH", "KeyD", "KeyN", "KeyT"].includes(code);
    const pageZoom = modifier && ["Equal", "Minus", "Digit0", "NumpadAdd", "NumpadSubtract", "Numpad0", "Key+", "Key-", "Key=", "Key0"].includes(code);
    const history = (event.altKey && ["ArrowLeft", "ArrowRight"].includes(key))
      || ["BrowserBack", "BrowserForward", "BrowserRefresh", "BrowserHome"].includes(key)
      || (key === "Backspace" && !editing);
    if (inspector || browserCommand || pageZoom || history || key === "F5") {
      event.preventDefault();
      event.stopPropagation();
      return;
    }
    if (modifier && code === "KeyA" && !editing && !event.shiftKey && !event.altKey) {
      event.preventDefault();
      event.stopPropagation();
      if (typeof CustomEvent === "function") window.dispatchEvent(new CustomEvent("k1tty:selectall", { detail: {} }));
      return;
    }
    if (modifier && !event.altKey && code === "KeyF") {
      event.preventDefault();
      if (!document.querySelector?.('.modal-bg.open, dialog[open]')) {
        window.dispatchEvent(new CustomEvent(event.shiftKey ? "k1tty:globalfind" : "k1tty:find", { detail: {} }));
      }
      return;
    }
    if (modifier && !event.shiftKey && !event.altKey && code === "KeyK") {
      event.preventDefault();
      if (!document.querySelector?.('dialog[open]')) {
        window.dispatchEvent(new CustomEvent("k1tty:palette", { detail: {} }));
      }
      return;
    }
  }, true);

  document.addEventListener("wheel", (event) => {
    if (event.ctrlKey || event.metaKey) event.preventDefault();
  }, { capture: true, passive: false });
   
  document.addEventListener("gesturestart", (event) => event.preventDefault(), { passive: false });
  document.addEventListener("gesturechange", (event) => event.preventDefault(), { passive: false });
  document.addEventListener("dragstart", (event) => {
    if (!isTextInput(event.target)) event.preventDefault();
  }, true);
  document.addEventListener("auxclick", (event) => {
    if (event.button === 1 && event.target?.closest?.("a[href]")) event.preventDefault();
  }, true);
  for (const type of ["dragover", "drop"]) {
    document.addEventListener(type, (event) => {
      if (!isTextInput(event.target) || event.dataTransfer?.files?.length) event.preventDefault();
       
    }, true);
  }

  const tauri = window.__TAURI__;
  if (!tauri?.core || !tauri?.event) return;
  const invoke = (command, args) => tauri.core.invoke(command, args).catch((err) => {
    throw err instanceof Error ? err : new Error(String(err));
  });
  const nativeWindow = tauri.window?.getCurrentWindow?.();
  const titlebar = document.getElementById("windowTitlebar");
  const minimizeBtn = document.getElementById("windowMinimize");
  const maximizeBtn = document.getElementById("windowMaximize");
  const closeBtn = document.getElementById("windowClose");
  let isMaximized = false;
  let isFullscreen = false;
  let windowLabels = { minimize: "Minimize", maximize: "Maximize", restore: "Restore", close: "Close" };
  let controlBusy = false;

  const dispatchError = (err) => {
    const message = err instanceof Error ? err.message : String(err);
    window.dispatchEvent(new CustomEvent("k1tty-error", { detail: message }));
    console.error("K1tty window action:", err);
  };

  const renderWindowLabels = () => {
    if (minimizeBtn) minimizeBtn.title = minimizeBtn.ariaLabel = windowLabels.minimize;
    if (closeBtn) closeBtn.title = closeBtn.ariaLabel = windowLabels.close;
    if (maximizeBtn) {
      const mode = isMaximized || isFullscreen ? "restore" : "maximize";
      maximizeBtn.title = maximizeBtn.ariaLabel = windowLabels[mode];
      maximizeBtn.dataset.maximized = String(mode === "restore");
      maximizeBtn.setAttribute("aria-pressed", String(mode === "restore"));
    }
  };

  const refreshWindowState = async () => {
    if (!nativeWindow) return;
    try {
      [isMaximized, isFullscreen] = await Promise.all([
        nativeWindow.isMaximized(),
        nativeWindow.isFullscreen(),
      ]);
      document.body.classList.toggle("window-maximized", isMaximized || isFullscreen);
      renderWindowLabels();
    } catch (error) {
      dispatchError(error);
    }
  };

  const doWindowAction = async (work) => {
    if (!nativeWindow || controlBusy) return;
    controlBusy = true;
    try {
      await work();
      await refreshWindowState();
    } catch (error) {
      dispatchError(error);
    } finally {
      controlBusy = false;
    }
  };

  minimizeBtn?.addEventListener("click", () => doWindowAction(() => nativeWindow.minimize()));
  maximizeBtn?.addEventListener("click", () => doWindowAction(async () => {
    if (await nativeWindow.isFullscreen()) await nativeWindow.setFullscreen(false);
    else await nativeWindow.toggleMaximize();
  }));
  closeBtn?.addEventListener("click", () => doWindowAction(() => nativeWindow.close()));

  document.addEventListener("keydown", (event) => {
    if (event.key !== "F11" || event.altKey || event.ctrlKey || event.metaKey || event.repeat) return;
    event.preventDefault();
    doWindowAction(async () => nativeWindow.setFullscreen(!(await nativeWindow.isFullscreen())));
  });

  for (const grip of document.querySelectorAll("[data-resize-direction]")) {
    grip.addEventListener("pointerdown", (event) => {
      if (event.button !== 0 || isMaximized || isFullscreen || !nativeWindow) return;
      event.preventDefault();
      nativeWindow.startResizeDragging(grip.dataset.resizeDirection).catch(dispatchError);
    });
  }

  nativeWindow?.onResized(() => refreshWindowState()).catch(dispatchError);
  nativeWindow?.onFocusChanged(({ payload }) => {
    titlebar?.classList.toggle("window-inactive", !payload);
    if (payload) refreshWindowState();
  }).catch(dispatchError);
  refreshWindowState();
  renderWindowLabels();

  window.K1ttyDesktop = Object.freeze({
    setWindowLabels: (labels) => {
      windowLabels = { ...windowLabels, ...labels };
      renderWindowLabels();
    },
    connect: async (onMessage) => {
      const unlisten = await tauri.event.listen("k1tty-message", ({ payload }) => onMessage(payload));
      try {
        const hello = await invoke("initialize");
        onMessage(hello);
      } catch (error) {
        unlisten();
        throw error;
      }
      return unlisten;
    },
    rpc: (op, provider, data) => invoke("rpc", { op, provider, data }),
    exportPack: (data) => invoke("export_pack", { data }),
    saveExport: (token) => invoke("save_export", { token }),
    exportBatch: (items, preset = "balanced", includeTools = true) => invoke("export_batch", { items, preset, includeTools }),
    inspectPack: (bytes) => invoke("inspect_pack", new Uint8Array(bytes)),
    importPack: (token, target, title, includeTools) => invoke("import_pack", { token, target, title, includeTools }),
    writeText: (text) => invoke("plugin:clipboard-manager|write_text", { text: String(text) }),
    openUrl: (url) => invoke("plugin:opener|open_url", { url }),
    pickFolder: () => invoke("pick_folder"),
    revealPath: (path) => invoke("reveal_path", { path: String(path) }),
  });
  document.addEventListener("click", (event) => {
    const link = event.target.closest?.("a[href]");
    if (!link) return;
    const rawHref = link.getAttribute("href") || "";
    if (link.hasAttribute("data-reveal-path")) {
      event.preventDefault();
      return;
    }
    if (/^(https?:|mailto:|file:)/i.test(rawHref)) {
      const sel = window.getSelection?.();
      let touched = false;
      try { touched = !!sel && !sel.isCollapsed && !!sel.toString() && sel.containsNode(link, true); } catch {}
      if (touched) {
        event.preventDefault();
        return;
      }
    }
    const token = rawHref.match(/^k1tty-export:([a-f0-9]+)$/)?.[1];
    if (token) {
      event.preventDefault();
      window.K1ttyDesktop.saveExport(token).catch((error) => window.dispatchEvent(new CustomEvent("k1tty-error", { detail: error.message })));
      return;
    }
    if (/^file:/i.test(rawHref)) {
      event.preventDefault();
      window.K1ttyDesktop.revealPath(rawHref).catch((error) => window.dispatchEvent(new CustomEvent("k1tty-error", { detail: error.message })));
      return;
    }
    if (/^(https?:|mailto:)/i.test(rawHref)) {
      event.preventDefault();
      window.K1ttyDesktop.openUrl(link.href).catch((error) => window.dispatchEvent(new CustomEvent("k1tty-error", { detail: error.message })));
    }
  });
})();
