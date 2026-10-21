// The small set of calls the page may make to the Windows app.
const { contextBridge, ipcRenderer } = require("electron");

contextBridge.exposeInMainWorld("clawdDesktop", {
  moveBy: (dx, dy) => ipcRenderer.send("move-by", dx, dy),
  dragEnd: () => ipcRenderer.send("drag-end"),
  setClickThrough: (through) => ipcRenderer.send("click-through", through),
  showMenu: () => ipcRenderer.send("menu"),
  setFullscreen: (on) => ipcRenderer.send("set-fullscreen", on),
  reportAuto: (on) => ipcRenderer.send("auto-changed", on),
  reportUltra: (on) => ipcRenderer.send("ultra-changed", on),
  onPlay: (fn) => ipcRenderer.on("play", (_e, name) => fn(name)),
  onAuto: (fn) => ipcRenderer.on("auto", (_e, on) => fn(on)),
  onUltra: (fn) => ipcRenderer.on("ultra", (_e, on) => fn(on)),
  onMode: (fn) => ipcRenderer.on("mode", (_e, mode) => fn(mode))
});
