// The desktop app's side of the page (apps/web/index.html): window.clawdDesktop, run in each window before the page.
// Calls to the app go through Tauri's invoke; the app calls back with window.__clawdHost(name, ...args).
// The Rust side puts the page's settings (the hash, e.g. "mode=pet&first=jump") in place of __CLAWD_HASH__.
(() => {
  history.replaceState(null, "", "#" + "__CLAWD_HASH__");
  const invoke = (cmd, args) => {
    const tauri = window.__TAURI_INTERNALS__;
    if (tauri) tauri.invoke(cmd, args || {}).catch(() => { /* the app is closing */ });
  };
  const handlers = {};
  const on = (name) => (fn) => { (handlers[name] = handlers[name] || []).push(fn); };
  window.__clawdHost = (name, ...args) => (handlers[name] || []).forEach((fn) => fn(...args));
  window.clawdDesktop = {
    moveBy: (dx, dy) => invoke("move_by", { dx, dy }),
    dragEnd: () => invoke("drag_end"),
    setHitArea: (rects, holding) => invoke("set_hit_area", { rects, holding }),
    showMenu: () => invoke("show_menu"),
    startFloating: (first, auto) => invoke("start_floating", { first: first || "", auto: !!auto }),
    stopFloating: () => invoke("stop_floating"),
    openApp: () => invoke("open_app"),
    quit: () => invoke("quit"),
    reportAuto: (on) => invoke("report_auto", { on: !!on }),
    reportUltra: (on) => invoke("report_ultra", { on: !!on }),
    sync: (message) => invoke("sync", { message }),
    checkUpdates: () => invoke("check_updates"),
    installUpdate: () => invoke("install_update"),
    onPlay: on("play"),
    onAuto: on("auto"),
    onUltra: on("ultra"),
    onFloating: on("floating"),
    onSync: on("sync"),
    onUpdate: on("update"),
    onClaude: on("claude"),
    onGaze: on("gaze")
  };
  // The page has loaded and listens: the app sends what it needs to know (floating, updates, the shared switches).
  addEventListener("DOMContentLoaded", () => setTimeout(() => invoke("page_ready"), 0));
})();
