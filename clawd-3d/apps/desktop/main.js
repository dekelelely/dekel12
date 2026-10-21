// Clawd for Windows: a borderless, always-on-top window in the corner of the screen with Clawd in it.
// Clicks pass through everywhere except Clawd himself. Tray icon and right-click menu for moves, size and fullscreen.
const { app, BrowserWindow, Tray, Menu, ipcMain, screen, nativeImage } = require("electron");
const path = require("path");
const fs = require("fs");

const MOVES = ["jump", "look", "celebrate", "skip", "spin", "peekaboo", "drop", "waddle", "peek", "wink", "boop", "tap", "sneeze", "turn", "coin-hop", "back-flip", "front-flip", "side-flip", "laptop", "ultracode"];
// Window widths; the window is wide and short (ASPECT) so his jumps and travel across the stage stay inside it.
const SIZES = { Small: 243, Medium: 343, Large: 471 };
const ASPECT = 1.54;
const MARGIN = 12;

let win = null;
let tray = null;
let full = false;
let petBounds = null;
let autoOn = false;
let ultraOn = false;
let settings = { size: "Medium", x: null, y: null };
const settingsFile = () => path.join(app.getPath("userData"), "settings.json");

function loadSettings() {
  try { settings = { ...settings, ...JSON.parse(fs.readFileSync(settingsFile(), "utf8")) }; } catch (err) { /* first run */ }
  if (!SIZES[settings.size]) settings.size = "Medium";
}
function saveSettings() {
  try { fs.writeFileSync(settingsFile(), JSON.stringify(settings)); } catch (err) { /* not fatal */ }
}

function petSize() {
  const w = SIZES[settings.size];
  return { width: w, height: Math.round(w / ASPECT) };
}
// Bottom-right of the screen's work area, so Clawd stands on the taskbar.
function cornerBounds(display = screen.getPrimaryDisplay()) {
  const { width, height } = petSize();
  const wa = display.workArea;
  return { x: wa.x + wa.width - width - MARGIN, y: wa.y + wa.height - height, width, height };
}
function savedBounds() {
  const { width, height } = petSize();
  if (settings.x === null || settings.y === null) return cornerBounds();
  // Keep the saved bottom-right corner if the window size changed since (a new version or another size).
  const b = {
    x: settings.x + (settings.width ? settings.width - width : 0),
    y: settings.y + (settings.height ? settings.height - height : 0),
    width,
    height
  };
  // Fall back to the corner if the saved spot is no longer on any screen.
  const d = screen.getDisplayMatching(b).workArea;
  const visible = b.x + width > d.x && b.x < d.x + d.width && b.y + height > d.y && b.y < d.y + d.height;
  return visible ? b : cornerBounds();
}

function createWindow() {
  const b = savedBounds();
  win = new BrowserWindow({
    ...b,
    transparent: true,
    frame: false,
    resizable: false,
    maximizable: false,
    fullscreenable: false,
    alwaysOnTop: true,
    skipTaskbar: true,
    hasShadow: false,
    show: false,
    backgroundColor: "#00000000",
    title: "Clawd",
    icon: path.join(__dirname, "build", "icon.png"),
    webPreferences: {
      preload: path.join(__dirname, "preload.js"),
      contextIsolation: true,
      nodeIntegration: false,
      backgroundThrottling: false
    }
  });
  win.setAlwaysOnTop(true, "floating");
  win.setIgnoreMouseEvents(true, { forward: true });
  win.loadFile(path.join(__dirname, "web", "index.html"), { hash: "mode=pet&platform=electron" });
  win.once("ready-to-show", () => win.showInactive());
  win.on("closed", () => { win = null; });
}

function setFull(on) {
  if (!win || on === full) return;
  full = on;
  if (on) {
    petBounds = win.getBounds();
    const display = screen.getDisplayMatching(petBounds);
    win.setIgnoreMouseEvents(false);
    win.setSkipTaskbar(false);
    win.setResizable(true);
    win.setBounds(display.bounds);
    win.setAlwaysOnTop(true, "screen-saver");
    win.focus();
  } else {
    win.setAlwaysOnTop(true, "floating");
    win.setBounds(petBounds || savedBounds());
    win.setResizable(false);
    win.setSkipTaskbar(true);
    win.setIgnoreMouseEvents(true, { forward: true });
  }
  win.webContents.send("mode", on ? "full" : "pet");
  refreshTray();
}

function setSize(name) {
  settings.size = name;
  if (win && !full) {
    // Keep his feet where they were: anchor the bottom-right corner.
    const old = win.getBounds();
    const { width, height } = petSize();
    win.setBounds({ x: old.x + old.width - width, y: old.y + old.height - height, width, height });
    rememberPosition();
  }
  saveSettings();
  refreshTray();
}

function rememberPosition() {
  if (!win || full) return;
  const { x, y, width, height } = win.getBounds();
  Object.assign(settings, { x, y, width, height });
  saveSettings();
}

function toCorner() {
  if (!win) return;
  if (full) setFull(false);
  win.setBounds(cornerBounds(screen.getDisplayMatching(win.getBounds())));
  rememberPosition();
}

function menuTemplate() {
  return [
    { label: "Play a move", submenu: MOVES.map((name) => ({ label: name, click: () => win && win.webContents.send("play", name) })) },
    { label: "Auto-play every move", type: "checkbox", checked: autoOn, click: (item) => win && win.webContents.send("auto", item.checked) },
    { label: "Ultra effort (electric guitar)", type: "checkbox", checked: ultraOn, click: (item) => win && win.webContents.send("ultra", item.checked) },
    { type: "separator" },
    { label: full ? "Back to corner" : "Fullscreen", click: () => setFull(!full) },
    { label: "Size", submenu: Object.keys(SIZES).map((name) => ({ label: name, type: "radio", checked: settings.size === name, click: () => setSize(name) })) },
    { label: "Move to corner", click: toCorner },
    { label: "Start with Windows", type: "checkbox", checked: app.getLoginItemSettings().openAtLogin, click: (item) => app.setLoginItemSettings({ openAtLogin: item.checked }) },
    { type: "separator" },
    { label: "Quit Clawd", click: () => app.quit() }
  ];
}
function refreshTray() {
  if (tray) tray.setContextMenu(Menu.buildFromTemplate(menuTemplate()));
}

ipcMain.on("move-by", (_e, dx, dy) => {
  if (!win || full) return;
  const [x, y] = win.getPosition();
  win.setPosition(Math.round(x + dx), Math.round(y + dy));
});
ipcMain.on("drag-end", rememberPosition);
ipcMain.on("click-through", (_e, through) => {
  if (win && !full) win.setIgnoreMouseEvents(!!through, { forward: true });
});
ipcMain.on("menu", () => {
  if (win) Menu.buildFromTemplate(menuTemplate()).popup({ window: win });
});
ipcMain.on("set-fullscreen", (_e, on) => setFull(!!on));
ipcMain.on("auto-changed", (_e, on) => { autoOn = !!on; refreshTray(); });
ipcMain.on("ultra-changed", (_e, on) => { ultraOn = !!on; refreshTray(); });

if (!app.requestSingleInstanceLock()) {
  app.quit();
} else {
  app.on("second-instance", () => { if (win) win.showInactive(); });
  app.whenReady().then(() => {
    app.setAppUserModelId("com.dekelelely.clawd");
    loadSettings();
    createWindow();
    const icon = nativeImage.createFromPath(path.join(__dirname, "build", "tray.png"));
    tray = new Tray(icon.isEmpty() ? nativeImage.createEmpty() : icon);
    tray.setToolTip("Clawd");
    tray.on("click", () => setFull(!full));
    refreshTray();
    screen.on("display-metrics-changed", () => { if (win && !full) win.setBounds(savedBounds()); });
  });
  app.on("window-all-closed", () => app.quit());
}
