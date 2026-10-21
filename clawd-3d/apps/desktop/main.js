// Clawd for Windows, built like the Android app: an app window with every move and a "Float on screen" button, and
// floating Clawd, a borderless always-on-top window on the desktop whose clicks pass through everywhere except on
// Clawd himself. A tray icon and right-click menu hold the moves, the floating size, start with Windows and updates.
const { app, BrowserWindow, Tray, Menu, ipcMain, screen, nativeImage, shell, dialog } = require("electron");
const path = require("path");
const fs = require("fs");

const MOVES = ["jump", "look", "celebrate", "skip", "spin", "peekaboo", "drop", "waddle", "peek", "wink", "boop", "tap", "sneeze", "turn", "coin-hop", "back-flip", "front-flip", "side-flip", "dumbbell-curl", "laptop", "ultracode"];
// Floating window widths; the window is wide and short (ASPECT) so his jumps and travel across the stage stay inside it.
const SIZES = { Small: 243, Medium: 343, Large: 471 };
const ASPECT = 1.54;
const MARGIN = 12;
const AUTOSTART = "--autostart";   // how Windows starts Clawd at login: floating straight away, no app window
const LOGIN_PATH = process.env.PORTABLE_EXECUTABLE_FILE || process.execPath;
const PAGE = path.join(__dirname, "web", "index.html");
const ICON = path.join(__dirname, "build", "icon.png");

let appWin = null;   // the app: every move, Auto-play, Float on screen, updates
let petWin = null;   // floating Clawd
let tray = null;
let autoOn = false;
let ultraOn = false;
let settings = { size: "Medium", x: null, y: null, app: null };
const settingsFile = () => path.join(app.getPath("userData"), "settings.json");

function loadSettings() {
  try { settings = { ...settings, ...JSON.parse(fs.readFileSync(settingsFile(), "utf8")) }; } catch (err) { /* first run */ }
  if (!SIZES[settings.size]) settings.size = "Medium";
}
function saveSettings() {
  try { fs.writeFileSync(settingsFile(), JSON.stringify(settings)); } catch (err) { /* not fatal */ }
}
const onScreen = (b) => {
  const d = screen.getDisplayMatching(b).workArea;
  return b.x + b.width > d.x && b.x < d.x + d.width && b.y + b.height > d.y && b.y < d.y + d.height;
};
const windows = () => [appWin, petWin].filter(Boolean);
function sendAll(channel, ...args) {
  for (const w of windows()) w.webContents.send(channel, ...args);
}

// ---- The app window ----
function showApp() {
  refreshUpdates();
  if (appWin) {
    if (appWin.isMinimized()) appWin.restore();
    appWin.show();
    appWin.focus();
    return;
  }
  const saved = settings.app;
  const b = saved && onScreen(saved) ? saved : { width: 960, height: 620 };
  appWin = new BrowserWindow({
    ...b,
    minWidth: 420,
    minHeight: 360,
    show: false,
    backgroundColor: "#0d0f12",
    title: "Clawd",
    icon: ICON,
    webPreferences: {
      preload: path.join(__dirname, "preload.js"),
      contextIsolation: true,
      nodeIntegration: false
    }
  });
  if (saved && saved.maximized) appWin.maximize();
  appWin.loadFile(PAGE, { hash: "mode=full&platform=electron" });
  appWin.once("ready-to-show", () => appWin.show());
  appWin.webContents.on("did-finish-load", () => sendState(appWin));
  appWin.on("close", () => {
    settings.app = { ...appWin.getNormalBounds(), maximized: appWin.isMaximized() };
    saveSettings();
  });
  appWin.on("closed", () => { appWin = null; });
}

// ---- Floating Clawd ----
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
  return onScreen(b) ? b : cornerBounds();
}

// first: a move to open with (the one last picked in the app), or "" for a random one. auto: start on Auto-play.
function startFloating(first = "", auto = false) {
  if (petWin) return;
  petWin = new BrowserWindow({
    ...savedBounds(),
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
    icon: ICON,
    webPreferences: {
      preload: path.join(__dirname, "preload.js"),
      contextIsolation: true,
      nodeIntegration: false,
      backgroundThrottling: false
    }
  });
  petWin.setAlwaysOnTop(true, "floating");
  petWin.setIgnoreMouseEvents(true, { forward: true });
  const hash = "mode=pet&platform=electron" + (/^[a-z]+(-[a-z]+)*$/.test(first) ? `&first=${first}` : "") + (auto ? "&auto=1" : "");
  petWin.loadFile(PAGE, { hash });
  petWin.once("ready-to-show", () => petWin.showInactive());
  petWin.webContents.on("did-finish-load", () => sendState(petWin));
  petWin.on("closed", () => {
    petWin = null;
    floatingChanged();
  });
  floatingChanged();
}
function stopFloating() {
  if (petWin) petWin.close();
}
function floatingChanged() {
  if (appWin) appWin.webContents.send("floating", !!petWin);
  refreshTray();
}

function setSize(name) {
  settings.size = name;
  if (petWin) {
    // Keep his feet where they were: anchor the bottom-right corner.
    const old = petWin.getBounds();
    const { width, height } = petSize();
    petWin.setBounds({ x: old.x + old.width - width, y: old.y + old.height - height, width, height });
    rememberPosition();
  }
  saveSettings();
  refreshTray();
}
function rememberPosition() {
  if (!petWin) return;
  const { x, y, width, height } = petWin.getBounds();
  Object.assign(settings, { x, y, width, height });
  saveSettings();
}
function toCorner() {
  if (!petWin) return;
  petWin.setBounds(cornerBounds(screen.getDisplayMatching(petWin.getBounds())));
  rememberPosition();
}

// What a page needs when it loads: whether Clawd is floating, the update button's state, and (for an app window
// opened while he floats) the Auto-play switch the two share.
function sendState(w) {
  w.webContents.send("floating", !!petWin);
  w.webContents.send("update-state", update);
  if (w === appWin && petWin) w.webContents.send("sync", { auto: autoOn });
}

// ---- Start with Windows ----
// At login Clawd starts floating, without the app window.
function setStartWithWindows(on) {
  settings.startWithWindows = on;
  saveSettings();
  app.setLoginItemSettings({ openAtLogin: on, path: LOGIN_PATH, args: [AUTOSTART] });
  refreshTray();
}
function initStartWithWindows() {
  // Versions before the app window registered the login item without AUTOSTART, which would now open the app window.
  if (settings.startWithWindows === undefined) setStartWithWindows(app.getLoginItemSettings({ path: LOGIN_PATH }).openAtLogin);
}

// ---- Updates ----
// The "Clawd apps" workflow publishes every build as a GitHub Release. The installed app checks when it opens and
// from the menu, downloads a new version in the background, and installs it on restart (or whenever Clawd quits).
// The portable .exe can't replace itself, so there "updates" opens the releases page instead.
const RELEASES_URL = "https://github.com/dekelelely/dekel12/releases/latest";
const shortVersion = (v) => String(v || "").replace(/\.0$/, "");
let updater = null;
let update = { state: "off" };   // off, portable, idle, checking, none, downloading, ready or error
let checkFromMenu = false;
let lastCheck = 0;

function setUpdate(next) {
  update = { current: app.getVersion(), ...next };
  sendAll("update-state", update);
  refreshTray();
}
function initUpdates() {
  if (!app.isPackaged) return;   // npm start
  if (process.env.PORTABLE_EXECUTABLE_FILE) return setUpdate({ state: "portable" });
  updater = require("electron-updater").autoUpdater;
  updater.autoDownload = true;
  updater.autoInstallOnAppQuit = true;
  updater.on("checking-for-update", () => setUpdate({ state: "checking" }));
  updater.on("update-available", (info) => setUpdate({ state: "downloading", version: info.version, percent: 0 }));
  updater.on("download-progress", (p) => setUpdate({ state: "downloading", version: update.version, percent: Math.floor(p.percent) }));
  updater.on("update-downloaded", (info) => setUpdate({ state: "ready", version: info.version }));
  updater.on("update-not-available", () => {
    setUpdate({ state: "none" });
    if (checkFromMenu) dialog.showMessageBox({ title: "Clawd", message: `Clawd ${shortVersion(app.getVersion())} is the latest version.` });
    checkFromMenu = false;
  });
  updater.on("error", (err) => {
    const message = String((err && err.message) || err).split("\n")[0];
    setUpdate({ state: "error", message });
    if (checkFromMenu) dialog.showMessageBox({ type: "warning", title: "Clawd", message: "Couldn't check for updates.", detail: message });
    checkFromMenu = false;
  });
  setUpdate({ state: "idle" });
  checkUpdates(false);
  setInterval(refreshUpdates, 60 * 60 * 1000);   // Clawd may float all day: look again every hour
}
function checkUpdates(fromMenu) {
  if (update.state === "portable") return shell.openExternal(RELEASES_URL);
  if (!updater || ["checking", "downloading", "ready"].includes(update.state)) return;
  checkFromMenu = fromMenu;
  lastCheck = Date.now();
  updater.checkForUpdates().catch(() => { /* reported through the "error" event */ });
}
// A quiet check, when the app window comes up or the hour is up, unless the last one was under a minute ago.
function refreshUpdates() {
  if (updater && Date.now() - lastCheck > 60 * 1000) checkUpdates(false);
}
function installUpdate() {
  if (updater && update.state === "ready") updater.quitAndInstall(true, true);   // silent install, then start again
}
function updateMenuItem() {
  const v = shortVersion(update.version);
  switch (update.state) {
    case "off": return null;
    case "portable": return { label: "Get the latest version…", click: () => checkUpdates(true) };
    case "checking": return { label: "Checking for updates…", enabled: false };
    case "downloading": return { label: `Downloading Clawd ${v}… ${update.percent || 0}%`, enabled: false };
    case "ready": return { label: `Restart to update to ${v}`, click: installUpdate };
    default: return { label: `Check for updates (Clawd ${shortVersion(app.getVersion())})`, click: () => checkUpdates(true) };
  }
}

// ---- Tray and right-click menu ----
// Moves and auto-play go to floating Clawd while he is out, otherwise to the app window.
const moveTarget = () => petWin || appWin;
function menuTemplate() {
  const updateItem = updateMenuItem();
  return [
    { label: "Open Clawd", click: showApp },
    { label: petWin ? "Hide floating Clawd" : "Float on screen", click: () => (petWin ? stopFloating() : startFloating("", autoOn)) },
    { type: "separator" },
    { label: "Play a move", submenu: MOVES.map((name) => ({ label: name, click: () => moveTarget() && moveTarget().webContents.send("play", name) })) },
    { label: "Auto-play random moves", type: "checkbox", checked: autoOn, click: (item) => moveTarget() && moveTarget().webContents.send("auto", item.checked) },
    { label: "Ultra effort (electric guitar)", type: "checkbox", checked: ultraOn, click: (item) => sendAll("ultra", item.checked) },
    { type: "separator" },
    { label: "Floating size", submenu: Object.keys(SIZES).map((name) => ({ label: name, type: "radio", checked: settings.size === name, click: () => setSize(name) })) },
    { label: "Move to corner", enabled: !!petWin, click: toCorner },
    { label: "Start with Windows", type: "checkbox", checked: !!settings.startWithWindows, click: (item) => setStartWithWindows(item.checked) },
    { type: "separator" },
    ...(updateItem ? [updateItem] : []),
    { label: "Quit Clawd", click: () => app.quit() }
  ];
}
function refreshTray() {
  if (tray) tray.setContextMenu(Menu.buildFromTemplate(menuTemplate()));
}

// ---- Calls from the pages (preload.js) ----
const fromPet = (e) => petWin && e.sender === petWin.webContents;
ipcMain.on("move-by", (e, dx, dy) => {
  if (!fromPet(e)) return;
  const [x, y] = petWin.getPosition();
  petWin.setPosition(Math.round(x + dx), Math.round(y + dy));
});
ipcMain.on("drag-end", (e) => { if (fromPet(e)) rememberPosition(); });
ipcMain.on("click-through", (e, through) => { if (fromPet(e)) petWin.setIgnoreMouseEvents(!!through, { forward: true }); });
ipcMain.on("menu", (e) => {
  const w = BrowserWindow.fromWebContents(e.sender);
  if (w) Menu.buildFromTemplate(menuTemplate()).popup({ window: w });
});
ipcMain.on("start-floating", (_e, first, auto) => startFloating(typeof first === "string" ? first : "", auto === true));
// The app window and floating Clawd stay matched: each one's moves and Auto-play switch go to the other.
ipcMain.on("sync", (e, message) => {
  if (!message || typeof message !== "object") return;
  const m = {};
  if (typeof message.move === "string" && MOVES.includes(message.move)) m.move = message.move;
  if (typeof message.auto === "boolean") m.auto = message.auto;
  const other = appWin && e.sender === appWin.webContents ? petWin : petWin && e.sender === petWin.webContents ? appWin : null;
  if (other && Object.keys(m).length) other.webContents.send("sync", m);
});
ipcMain.on("stop-floating", stopFloating);
ipcMain.on("open-app", showApp);
ipcMain.on("quit", () => app.quit());
ipcMain.on("auto-changed", (_e, on) => { autoOn = !!on; refreshTray(); });
ipcMain.on("ultra-changed", (_e, on) => { ultraOn = !!on; refreshTray(); });
ipcMain.on("check-updates", () => checkUpdates(false));
ipcMain.on("install-update", installUpdate);

if (!app.requestSingleInstanceLock()) {
  app.quit();
} else {
  // Opening Clawd again (Start menu, desktop shortcut) brings up the app window, as on Android.
  app.on("second-instance", (_e, argv) => { if (!argv.includes(AUTOSTART)) showApp(); });
  app.whenReady().then(() => {
    app.setAppUserModelId("com.dekelelely.clawd");
    Menu.setApplicationMenu(null);
    loadSettings();
    initStartWithWindows();
    const icon = nativeImage.createFromPath(path.join(__dirname, "build", "tray.png"));
    tray = new Tray(icon.isEmpty() ? nativeImage.createEmpty() : icon);
    tray.setToolTip("Clawd");
    tray.on("click", showApp);
    if (process.argv.includes(AUTOSTART)) startFloating();
    else showApp();
    refreshTray();
    initUpdates();
    screen.on("display-metrics-changed", () => { if (petWin) petWin.setBounds(savedBounds()); });
  });
  // Closing the app window while Clawd floats keeps him out; once neither is open, Clawd quits (like Android).
  app.on("window-all-closed", () => app.quit());
}
