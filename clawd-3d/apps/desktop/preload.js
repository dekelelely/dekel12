// The small set of calls the page may make to the Windows app.
const { contextBridge, ipcRenderer } = require("electron");

contextBridge.exposeInMainWorld("clawdDesktop", {
  moveBy: (dx, dy) => ipcRenderer.send("move-by", dx, dy),
  dragEnd: () => ipcRenderer.send("drag-end"),
  setClickThrough: (through) => ipcRenderer.send("click-through", through),
  showMenu: () => ipcRenderer.send("menu"),
  startFloating: (first, auto) => ipcRenderer.send("start-floating", first, auto),
  stopFloating: () => ipcRenderer.send("stop-floating"),
  openApp: () => ipcRenderer.send("open-app"),
  quit: () => ipcRenderer.send("quit"),
  reportAuto: (on) => ipcRenderer.send("auto-changed", on),
  reportUltra: (on) => ipcRenderer.send("ultra-changed", on),
  onPlay: (fn) => ipcRenderer.on("play", (_e, name) => fn(name)),
  onAuto: (fn) => ipcRenderer.on("auto", (_e, on) => fn(on)),
  onUltra: (fn) => ipcRenderer.on("ultra", (_e, on) => fn(on)),
  onFloating: (fn) => ipcRenderer.on("floating", (_e, on) => fn(on)),
  sync: (message) => ipcRenderer.send("sync", message),
  onSync: (fn) => ipcRenderer.on("sync", (_e, message) => fn(message)),
  checkUpdates: () => ipcRenderer.send("check-updates"),
  installUpdate: () => ipcRenderer.send("install-update"),
  onUpdate: (fn) => ipcRenderer.on("update-state", (_e, state) => fn(state))
});
