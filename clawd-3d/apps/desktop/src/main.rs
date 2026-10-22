// Clawd for Windows, macOS and Linux, like the Android app: an app window with every move and a "Float on screen"
// button, and floating Clawd, a borderless always-on-top window on the desktop whose clicks pass through everywhere
// except on Clawd himself. A tray icon and right-click menu hold the moves, the floating size, start at login,
// Claude Code and updates. Both windows show the page in apps/web; src/bridge.js connects it to this app.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod claude;
mod settings;
mod updates;

use serde::Deserialize;
use serde_json::{json, Value};
use settings::{AppBounds, Settings, SIZES};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};
use tauri::menu::{CheckMenuItem, IsMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::webview::PageLoadEvent;
use tauri::window::Color;
use tauri::{
    AppHandle, LogicalPosition, LogicalSize, Manager, Monitor, PhysicalPosition, RunEvent, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder, WindowEvent, Wry,
};
use tauri_plugin_autostart::ManagerExt as _;
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use updates::{Install, Status};

// Every move in clawd-engine.js, by name (build.rs reads them from rust/clawd-core/moves.json).
include!(concat!(env!("OUT_DIR"), "/moves.rs"));
const ASPECT: f64 = 1.54; // floating window width / height: room for his jumps and travel
const MARGIN: f64 = 12.0;
const AUTOSTART: &str = "--autostart"; // how Clawd starts at login: floating straight away, no app window
const BRIDGE: &str = include_str!("bridge.js");
// WebView2 (Windows) takes these for every window alike: Tauri's defaults, and sound that may start without a click
// (the ultracode riff when a move comes from the menu).
const BROWSER_ARGS: &str =
    "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection --autoplay-policy=no-user-gesture-required";
#[cfg(target_os = "windows")]
const LOGIN_LABEL: &str = "Start with Windows";
#[cfg(target_os = "macos")]
const LOGIN_LABEL: &str = "Open at login";
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
const LOGIN_LABEL: &str = "Start at login";
/// Claude Code's "working" ends with Stop, but not when you interrupt it; after this long without a hook he stops.
const WORK_TIMEOUT: Duration = Duration::from_secs(10 * 60);

/// A box of Clawd's body on screen, in the floating page's CSS pixels.
#[derive(Clone, Copy, Debug, Deserialize)]
struct Rect {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

impl Rect {
    fn contains(&self, x: f64, y: f64, margin: f64) -> bool {
        x >= self.x - margin && x <= self.x + self.w + margin && y >= self.y - margin && y <= self.y + self.h + margin
    }
}

struct Shared {
    settings: Settings,
    config_dir: PathBuf,
    auto_on: bool, // the Auto-play switch the app window and floating Clawd share
    ultra_on: bool,
    hit: Vec<Rect>, // floating Clawd's body
    holding: bool,  // a drag or a double or triple click is under way: keep taking clicks
    ignoring: bool, // floating Clawd's window lets clicks through
    update: Status,
    install: Install,
    update_file: Option<PathBuf>, // a downloaded update waiting to be installed
    last_check: Option<Instant>,
    claude_hooks: bool,
    working_since: Option<Instant>,
}

struct Clawd(Mutex<Shared>);

fn shared(app: &AppHandle) -> MutexGuard<'_, Shared> {
    app.state::<Clawd>().inner().0.lock().unwrap_or_else(|e| e.into_inner())
}

fn save_settings(app: &AppHandle) {
    let s = shared(app);
    s.settings.save(&s.config_dir);
}

fn app_win(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window("app")
}

fn pet_win(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window("pet")
}

/// Calls the page: window.__clawdHost(name, ...args), as set up by bridge.js.
fn call(w: &WebviewWindow, name: &str, args: &[Value]) {
    let mut js = format!("window.__clawdHost && window.__clawdHost({}", json!(name));
    for a in args {
        js.push(',');
        js.push_str(&a.to_string());
    }
    js.push(')');
    let _ = w.eval(&js);
}

fn call_all(app: &AppHandle, name: &str, args: &[Value]) {
    for w in [app_win(app), pet_win(app)].into_iter().flatten() {
        call(&w, name, args);
    }
}

/// Moves, Auto-play and Claude Code go to floating Clawd while he is out, otherwise to the app window; the other one
/// follows along.
fn move_target(app: &AppHandle) -> Option<WebviewWindow> {
    pet_win(app).or_else(|| app_win(app))
}

fn page_script(hash: &str) -> String {
    BRIDGE.replace("__CLAWD_HASH__", hash)
}

fn valid_move(name: &str) -> bool {
    MOVES.contains(&name)
}

// ---- Screens ----

/// A monitor's work area (without the taskbar or the menu bar and dock) in logical pixels: x, y, width, height.
fn work_area(m: &Monitor) -> (f64, f64, f64, f64) {
    let s = m.scale_factor();
    let wa = m.work_area();
    (wa.position.x as f64 / s, wa.position.y as f64 / s, wa.size.width as f64 / s, wa.size.height as f64 / s)
}

/// Whether a window at these logical bounds would be at least partly on a screen.
fn on_screen(app: &AppHandle, x: f64, y: f64, w: f64, h: f64) -> bool {
    app.available_monitors().unwrap_or_default().iter().any(|m| {
        let (mx, my, mw, mh) = work_area(m);
        x + w > mx && x < mx + mw && y + h > my && y < my + mh
    })
}

// ---- The app window ----

fn show_app(app: &AppHandle) {
    refresh_updates(app);
    if let Some(w) = app_win(app) {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
        return;
    }
    let saved = shared(app).settings.app.clone().filter(|b| on_screen(app, b.x, b.y, b.width, b.height));
    let mut builder = WebviewWindowBuilder::new(app, "app", WebviewUrl::App("index.html".into()))
        .title("Clawd")
        .min_inner_size(420.0, 360.0)
        .background_color(Color(13, 15, 18, 255))
        .initialization_script(page_script("mode=full&platform=desktop"))
        .additional_browser_args(BROWSER_ARGS)
        .visible(false)
        .on_page_load(|w, payload| {
            if payload.event() == PageLoadEvent::Finished {
                let _ = w.show();
                let _ = w.set_focus();
            }
        });
    builder = match &saved {
        Some(b) => builder.inner_size(b.width, b.height).position(b.x, b.y).maximized(b.maximized),
        None => builder.inner_size(960.0, 620.0).center(),
    };
    let w = match builder.build() {
        Ok(w) => w,
        Err(err) => return eprintln!("Clawd: couldn't open the app window: {err}"),
    };
    show_in_dock(app, true);
    let handle = app.clone();
    w.on_window_event(move |event| match event {
        WindowEvent::Moved(_) | WindowEvent::Resized(_) => remember_app_bounds(&handle),
        WindowEvent::CloseRequested { .. } => save_settings(&handle),
        WindowEvent::Destroyed => show_in_dock(&handle, false),
        _ => {}
    });
}

/// macOS: Clawd is in the Dock while the app window is open; floating on his own he is only in the menu bar, as his
/// window stays off the taskbar on Windows.
fn show_in_dock(app: &AppHandle, visible: bool) {
    #[cfg(target_os = "macos")]
    let _ = app.set_dock_visibility(visible);
    #[cfg(not(target_os = "macos"))]
    let _ = (app, visible);
}

/// The app window's size and spot while it isn't maximized, and whether it is, for next time (saved when it
/// closes or Clawd quits).
fn remember_app_bounds(app: &AppHandle) {
    let Some(w) = app_win(app) else { return };
    if w.is_minimized().unwrap_or(false) {
        return;
    }
    let maximized = w.is_maximized().unwrap_or(false);
    let mut s = shared(app);
    if maximized {
        if let Some(b) = s.settings.app.as_mut() {
            b.maximized = true;
        }
        return;
    }
    drop(s);
    let (Ok(pos), Ok(size), Ok(scale)) = (w.outer_position(), w.inner_size(), w.scale_factor()) else { return };
    let (pos, size) = (pos.to_logical::<f64>(scale), size.to_logical::<f64>(scale));
    s = shared(app);
    s.settings.app = Some(AppBounds { x: pos.x, y: pos.y, width: size.width, height: size.height, maximized: false });
}

// ---- Floating Clawd ----

fn pet_size(s: &Settings) -> (f64, f64) {
    let w = s.pet_width();
    (w, (w / ASPECT).round())
}

/// Bottom-right of a screen's work area, so Clawd stands on the taskbar (or the dock).
fn corner_bounds(app: &AppHandle, monitor: Option<Monitor>) -> (f64, f64, f64, f64) {
    let (w, h) = pet_size(&shared(app).settings);
    let m = monitor.or_else(|| app.primary_monitor().ok().flatten());
    let (x, y, mw, mh) = m.as_ref().map(work_area).unwrap_or((0.0, 0.0, 1280.0, 720.0));
    (x + mw - w - MARGIN, y + mh - h, w, h)
}

fn saved_bounds(app: &AppHandle) -> (f64, f64, f64, f64) {
    let s = shared(app).settings.clone();
    let (w, h) = pet_size(&s);
    let (Some(x), Some(y)) = (s.x, s.y) else { return corner_bounds(app, None) };
    // Keep the saved bottom-right corner if the window size changed since (another size or version).
    let x = x + s.width.map_or(0.0, |sw| sw - w);
    let y = y + s.height.map_or(0.0, |sh| sh - h);
    // Back to the corner if the saved spot is no longer on any screen.
    if on_screen(app, x, y, w, h) {
        (x, y, w, h)
    } else {
        corner_bounds(app, None)
    }
}

/// first: a move to open with (the one last picked in the app), or "" for a random one. auto: start on Auto-play.
fn start_floating(app: &AppHandle, first: &str, auto: bool) {
    if pet_win(app).is_some() {
        return;
    }
    let (x, y, w, h) = saved_bounds(app);
    let mut hash = "mode=pet&platform=desktop".to_string();
    if valid_move(first) {
        hash += &format!("&first={first}");
    }
    if auto {
        hash += "&auto=1";
    }
    let built = WebviewWindowBuilder::new(app, "pet", WebviewUrl::App("index.html".into()))
        .title("Clawd")
        .position(x, y)
        .inner_size(w, h)
        .transparent(true)
        .decorations(false)
        .resizable(false)
        .maximizable(false)
        .minimizable(false)
        .always_on_top(true)
        .visible_on_all_workspaces(true)
        .skip_taskbar(true)
        .shadow(false)
        .focused(false)
        .accept_first_mouse(true)
        .initialization_script(page_script(&hash))
        .additional_browser_args(BROWSER_ARGS)
        .build();
    let pet = match built {
        Ok(pet) => pet,
        Err(err) => return eprintln!("Clawd: couldn't float: {err}"),
    };
    let _ = pet.set_ignore_cursor_events(true);
    {
        let mut s = shared(app);
        s.ignoring = true;
        s.hit.clear();
        s.holding = false;
    }
    let handle = app.clone();
    pet.on_window_event(move |event| {
        if let WindowEvent::Destroyed = event {
            floating_changed(&handle, false);
        }
    });
    floating_changed(app, true);
}

fn stop_floating(app: &AppHandle) {
    if let Some(pet) = pet_win(app) {
        let _ = pet.destroy();
    }
}

fn floating_changed(app: &AppHandle, on: bool) {
    if let Some(w) = app_win(app) {
        call(&w, "floating", &[json!(on)]);
    }
    refresh_tray(app);
}

/// floating Clawd's window, in logical pixels.
fn pet_bounds(pet: &WebviewWindow) -> Option<(f64, f64, f64, f64)> {
    let scale = pet.scale_factor().ok()?;
    let pos = pet.outer_position().ok()?.to_logical::<f64>(scale);
    let size = pet.inner_size().ok()?.to_logical::<f64>(scale);
    Some((pos.x, pos.y, size.width, size.height))
}

/// Moves and sizes floating Clawd's window and remembers where (the window manager may take a moment to report it).
fn set_pet_bounds(app: &AppHandle, pet: &WebviewWindow, x: f64, y: f64, w: f64, h: f64) {
    let _ = pet.set_size(LogicalSize::new(w, h));
    let _ = pet.set_position(LogicalPosition::new(x, y));
    remember_bounds(app, x, y, w, h);
}

fn remember_bounds(app: &AppHandle, x: f64, y: f64, w: f64, h: f64) {
    {
        let mut s = shared(app);
        s.settings.x = Some(x);
        s.settings.y = Some(y);
        s.settings.width = Some(w);
        s.settings.height = Some(h);
    }
    save_settings(app);
}

/// After a drag: where he was put down.
fn remember_position(app: &AppHandle) {
    if let Some((x, y, w, h)) = pet_win(app).as_ref().and_then(pet_bounds) {
        remember_bounds(app, x, y, w, h);
    }
}

fn set_size(app: &AppHandle, name: &str) {
    shared(app).settings.size = name.to_string();
    if let Some(pet) = pet_win(app) {
        // Keep his feet where they were: anchor the bottom-right corner.
        if let Some((x, y, old_w, old_h)) = pet_bounds(&pet) {
            let (w, h) = pet_size(&shared(app).settings);
            set_pet_bounds(app, &pet, x + old_w - w, y + old_h - h, w, h);
        }
    }
    save_settings(app);
    refresh_tray(app);
}

fn to_corner(app: &AppHandle) {
    let Some(pet) = pet_win(app) else { return };
    let (x, y, w, h) = corner_bounds(app, pet.current_monitor().ok().flatten());
    set_pet_bounds(app, &pet, x, y, w, h);
}

/// Floating Clawd's window lets clicks through except over him: about 30 times a second while the mouse is near him
/// (10 when it is far away), it is checked against his body as the page last reported it. His eyes follow the mouse
/// too, until it rests for a few seconds.
fn watch_pointer(app: AppHandle) {
    std::thread::spawn(move || {
        let mut last_gaze: Option<(f64, f64)> = None;
        let mut moved_at = Instant::now();
        let mut resting = false; // the pointer stopped and he was told to look ahead
        let mut tick: u32 = 0;
        let mut near = true;
        loop {
            std::thread::sleep(Duration::from_millis(if near { 33 } else { 100 }));
            tick = tick.wrapping_add(if near { 1 } else { 3 });
            claude_timeout(&app);
            let Some(pet) = pet_win(&app) else {
                last_gaze = None;
                std::thread::sleep(Duration::from_millis(200));
                continue;
            };
            let (Ok(cursor), Ok(pos), Ok(scale)) = (app.cursor_position(), pet.outer_position(), pet.scale_factor())
            else {
                continue;
            };
            let (x, y) = ((cursor.x - pos.x as f64) / scale, (cursor.y - pos.y as f64) / scale);
            let (w, h) =
                pet.inner_size().map(|s| (s.width as f64 / scale, s.height as f64 / scale)).unwrap_or((0.0, 0.0));
            near = x > -150.0 && y > -150.0 && x < w + 150.0 && y < h + 150.0;
            let change = {
                let mut s = shared(&app);
                let over = s.holding || s.hit.iter().any(|r| r.contains(x, y, 2.0));
                let change = over == s.ignoring;
                s.ignoring = !over;
                change.then_some(!over)
            };
            if let Some(ignore) = change {
                let _ = pet.set_ignore_cursor_events(ignore);
            }
            // His eyes: the pointer in his page's pixels, 15 times a second while it moves; ahead once it rests.
            if !near || tick.is_multiple_of(2) {
                let moved = last_gaze.is_none_or(|(gx, gy)| (gx - x).abs() > 1.0 || (gy - y).abs() > 1.0);
                if moved {
                    moved_at = Instant::now();
                    last_gaze = Some((x, y));
                    resting = false;
                    call(&pet, "gaze", &[json!(x.round()), json!(y.round())]);
                } else if !resting && moved_at.elapsed() > Duration::from_secs(4) {
                    resting = true;
                    call(&pet, "gaze", &[Value::Null, Value::Null]);
                }
            }
            // Every few seconds: if his screen went away (unplugged, resolution changed), back to a corner.
            if tick % 150 < if near { 1 } else { 3 } {
                if let Some((x, y, w, h)) = pet_bounds(&pet) {
                    if !on_screen(&app, x, y, w, h) {
                        let app2 = app.clone();
                        let _ = app.run_on_main_thread(move || to_corner(&app2));
                    }
                }
            }
        }
    });
}

// ---- Start at login ----

fn set_start_at_login(app: &AppHandle, on: bool) {
    let launcher = app.autolaunch();
    let result = if on { launcher.enable() } else { launcher.disable() };
    if let Err(err) = result {
        eprintln!("Clawd: couldn't change start at login: {err}");
    }
    shared(app).settings.start_at_login = Some(on);
    save_settings(app);
    refresh_tray(app);
}

/// Keeps the login item matching the setting, pointing at this copy of Clawd (it may have moved, or the Electron
/// version may have had the setting on).
fn init_start_at_login(app: &AppHandle) {
    if let Some(true) = shared(app).settings.start_at_login {
        let _ = app.autolaunch().enable();
    }
}

// ---- Claude Code ----

fn claude_event(app: &AppHandle, kind: &'static str) {
    if kind != "hello" {
        shared(app).working_since = (kind == "working").then(Instant::now);
    }
    if let Some(w) = move_target(app) {
        call(&w, "claude", &[json!(kind)]);
    }
}

fn claude_timeout(app: &AppHandle) {
    let expired = shared(app).working_since.is_some_and(|t| t.elapsed() > WORK_TIMEOUT);
    if expired {
        claude_event(app, "idle");
    }
}

fn claude_settings() -> Option<PathBuf> {
    claude::settings_path(std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).map(PathBuf::from))
}

fn claude_hooks_installed() -> bool {
    claude_settings().and_then(|p| claude::read_settings(&p).ok()).is_some_and(|s| claude::installed(&s))
}

fn toggle_claude_hooks(app: &AppHandle) {
    let Some(path) = claude_settings() else { return };
    let installed = shared(app).claude_hooks;
    let (app2, app3) = (app.clone(), app.clone());
    let path2 = path.clone();
    let change = move |add: bool| {
        let result = claude::read_settings(&path2).and_then(|mut s| {
            if add {
                claude::add_hooks(&mut s)?
            } else {
                claude::remove_hooks(&mut s)
            }
            claude::write_settings(&path2, &s)
        });
        if let Err(err) = result {
            app2.dialog().message(err).title("Clawd").kind(MessageDialogKind::Warning).show(|_| {});
        }
        shared(&app2).claude_hooks = claude_hooks_installed();
        refresh_tray(&app2);
    };
    if installed {
        change(false);
        return;
    }
    let message = format!(
        "Clawd adds a hook to Claude Code's settings ({}), so he types on his laptop while Claude works, celebrates when \
         it's done and jumps when it needs you.\n\nThe hook only tells Clawd on this computer (127.0.0.1:{}); a copy of the \
         file is kept next to it. Uncheck this item to take the hook out again.",
        path.display(),
        claude::PORT
    );
    app.dialog()
        .message(message)
        .title("React to Claude Code")
        .buttons(MessageDialogButtons::OkCancelCustom("Add the hook".into(), "Cancel".into()))
        .show(move |ok| {
            if ok {
                change(true);
            } else {
                refresh_tray(&app3);
            }
        });
}

// ---- Updates ----

fn set_update(app: &AppHandle, state: &'static str, version: &str, percent: u32, message: &str) {
    let status = Status {
        state,
        current: app.package_info().version.to_string(),
        version: version.to_string(),
        percent,
        message: message.to_string(),
    };
    shared(app).update = status.clone();
    call_all(app, "update", &[serde_json::to_value(&status).unwrap_or_default()]);
    refresh_tray(app);
}

fn init_updates(app: &AppHandle) {
    // The AppImage runtime names the file it runs from; Clawd replaces that file to update.
    let install = updates::detect(std::env::var_os("APPIMAGE").map(PathBuf::from));
    shared(app).install = install.clone();
    match install {
        Install::Dev => {}
        Install::Portable => set_update(app, "portable", "", 0, ""),
        _ => {
            set_update(app, "idle", "", 0, "");
            check_updates(app, false);
            // Clawd may float all day: look again every hour.
            let app = app.clone();
            std::thread::spawn(move || loop {
                std::thread::sleep(Duration::from_secs(60 * 60));
                refresh_updates(&app);
            });
        }
    }
}

/// A quiet check, when the app window comes up or the hour is up, unless the last one was under a minute ago.
fn refresh_updates(app: &AppHandle) {
    let recent = shared(app).last_check.is_some_and(|t| t.elapsed() < Duration::from_secs(60));
    if !recent {
        check_updates(app, false);
    }
}

fn check_updates(app: &AppHandle, from_menu: bool) {
    let (state, install) = {
        let s = shared(app);
        (s.update.state, s.install.clone())
    };
    if state == "portable" {
        return open_releases();
    }
    if matches!(install, Install::Dev | Install::Portable) || ["checking", "downloading", "ready"].contains(&state) {
        return;
    }
    shared(app).last_check = Some(Instant::now());
    set_update(app, "checking", "", 0, "");
    let app = app.clone();
    std::thread::spawn(move || {
        let result = updates::latest().and_then(|(version, assets)| {
            let current = updates::build_of(&app.package_info().version.to_string()).unwrap_or(0);
            if updates::build_of(&version).unwrap_or(0) <= current {
                return Ok(None);
            }
            let names: Vec<String> = assets.iter().map(|(n, _)| n.clone()).collect();
            let name = updates::asset_for(&install, &names, &version)
                .ok_or("the latest release has no build for this system")?;
            let url = assets.into_iter().find(|(n, _)| *n == name).map(|(_, u)| u).unwrap_or_default();
            Ok(Some((version, name, url)))
        });
        match result {
            Ok(None) => {
                set_update(&app, "none", "", 0, "");
                if from_menu {
                    let v = short_version(&app.package_info().version.to_string());
                    app.dialog().message(format!("Clawd {v} is the latest version.")).title("Clawd").show(|_| {});
                }
            }
            Ok(Some((version, name, url))) => download_update(&app, &install, &version, &name, &url),
            Err(err) => {
                set_update(&app, "error", "", 0, &err);
                if from_menu {
                    app.dialog()
                        .message(format!("Couldn't check for updates.\n\n{err}"))
                        .title("Clawd")
                        .kind(MessageDialogKind::Warning)
                        .show(|_| {});
                }
            }
        }
    });
}

fn download_update(app: &AppHandle, install: &Install, version: &str, name: &str, url: &str) {
    let dir = updates::staging_dir(install);
    let _ = std::fs::remove_dir_all(&dir);
    let file = dir.join(name);
    set_update(app, "downloading", version, 0, "");
    let result = std::fs::create_dir_all(&dir)
        .map_err(|e| e.to_string())
        .and_then(|_| updates::download(url, &file, |p| set_update(app, "downloading", version, p, "")))
        .and_then(|_| match install {
            Install::AppBundle(_) => updates::unpack_app(&file, &dir),
            Install::AppImage(_) => updates::make_executable(&file).map(|_| file.clone()),
            _ => Ok(file.clone()),
        });
    match result {
        Ok(ready) => {
            shared(app).update_file = Some(ready);
            set_update(app, "ready", version, 100, "");
        }
        Err(err) => set_update(app, "error", version, 0, &format!("Couldn't download the update: {err}")),
    }
}

/// "Restart to update": put the new version in place and start it. `relaunch` false: Clawd is quitting anyway.
fn install_update(app: &AppHandle, relaunch: bool) {
    let (state, install, file) = {
        let mut s = shared(app);
        (s.update.state, s.install.clone(), s.update_file.take())
    };
    let Some(file) = file.filter(|_| state == "ready") else { return };
    let result = match &install {
        Install::Installer => {
            // The installer closes Clawd if he is still running; --force-run starts him again afterwards.
            let mut cmd = std::process::Command::new(&file);
            cmd.arg("/S");
            if relaunch {
                cmd.arg("--force-run");
            }
            cmd.spawn().map(|_| ()).map_err(|e| e.to_string())
        }
        Install::AppBundle(bundle) => updates::swap(bundle, &file).and_then(|_| {
            if relaunch {
                std::process::Command::new("open").arg("-n").arg(bundle).spawn().map_err(|e| e.to_string())?;
            }
            Ok(())
        }),
        Install::AppImage(image) => updates::swap(image, &file).and_then(|_| {
            if relaunch {
                std::process::Command::new(image).spawn().map_err(|e| e.to_string())?;
            }
            Ok(())
        }),
        _ => return,
    };
    match result {
        Ok(()) if relaunch => app.exit(0),
        Ok(()) => {}
        Err(err) => set_update(app, "error", "", 0, &format!("Couldn't install the update: {err}")),
    }
}

/// The Releases page, in the system's browser.
fn open_releases() {
    let url = updates::RELEASES_URL;
    let result = if cfg!(windows) {
        std::process::Command::new("explorer").arg(url).spawn()
    } else if cfg!(target_os = "macos") {
        std::process::Command::new("open").arg(url).spawn()
    } else {
        std::process::Command::new("xdg-open").arg(url).spawn()
    };
    if let Err(err) = result {
        eprintln!("Clawd: couldn't open {url}: {err}");
    }
}

fn short_version(v: &str) -> String {
    v.strip_suffix(".0").unwrap_or(v).to_string()
}

fn update_menu_item(app: &AppHandle) -> Option<MenuItem<Wry>> {
    let status = shared(app).update.clone();
    let v = short_version(&status.version);
    let current = short_version(&app.package_info().version.to_string());
    let (label, enabled) = match status.state {
        "" | "off" => return None,
        "portable" => ("Get the latest version…".to_string(), true),
        "checking" => ("Checking for updates…".to_string(), false),
        "downloading" => (format!("Downloading Clawd {v}… {}%", status.percent), false),
        "ready" => (format!("Restart to update to {v}"), true),
        _ => (format!("Check for updates (Clawd {current})"), true),
    };
    MenuItem::with_id(app, "update", label, enabled, None::<&str>).ok()
}

// ---- Tray and right-click menu ----

fn build_menu(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let floating = pet_win(app).is_some();
    let (auto_on, ultra_on, size, hooks) = {
        let s = shared(app);
        (s.auto_on, s.ultra_on, s.settings.size.clone(), s.claude_hooks)
    };
    let login = app.autolaunch().is_enabled().unwrap_or(false);
    let moves: Vec<MenuItem<Wry>> = MOVES
        .iter()
        .map(|m| MenuItem::with_id(app, format!("play:{m}"), *m, true, None::<&str>))
        .collect::<Result<_, _>>()?;
    let move_refs: Vec<&dyn IsMenuItem<Wry>> = moves.iter().map(|m| m as &dyn IsMenuItem<Wry>).collect();
    let sizes: Vec<CheckMenuItem<Wry>> = SIZES
        .iter()
        .map(|(name, _)| CheckMenuItem::with_id(app, format!("size:{name}"), *name, true, size == *name, None::<&str>))
        .collect::<Result<_, _>>()?;
    let size_refs: Vec<&dyn IsMenuItem<Wry>> = sizes.iter().map(|m| m as &dyn IsMenuItem<Wry>).collect();
    let update = update_menu_item(app);

    let open = MenuItem::with_id(app, "open", "Open Clawd", true, None::<&str>)?;
    let float = MenuItem::with_id(
        app,
        "float",
        if floating { "Hide floating Clawd" } else { "Float on screen" },
        true,
        None::<&str>,
    )?;
    let play = Submenu::with_items(app, "Play a move", true, &move_refs)?;
    let auto = CheckMenuItem::with_id(app, "auto", "Auto-play random moves", true, auto_on, None::<&str>)?;
    let ultra = CheckMenuItem::with_id(app, "ultra", "Ultra effort (electric guitar)", true, ultra_on, None::<&str>)?;
    let size_menu = Submenu::with_items(app, "Floating size", true, &size_refs)?;
    let corner = MenuItem::with_id(app, "corner", "Move to corner", floating, None::<&str>)?;
    let login = CheckMenuItem::with_id(app, "login", LOGIN_LABEL, true, login, None::<&str>)?;
    let claude = CheckMenuItem::with_id(app, "claude", "React to Claude Code", true, hooks, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Clawd", true, None::<&str>)?;
    let (sep1, sep2, sep3) =
        (PredefinedMenuItem::separator(app)?, PredefinedMenuItem::separator(app)?, PredefinedMenuItem::separator(app)?);

    let mut items: Vec<&dyn IsMenuItem<Wry>> =
        vec![&open, &float, &sep1, &play, &auto, &ultra, &sep2, &size_menu, &corner, &login, &claude, &sep3];
    if let Some(u) = &update {
        items.push(u);
    }
    items.push(&quit);
    Menu::with_items(app, &items)
}

fn refresh_tray(app: &AppHandle) {
    let app2 = app.clone();
    // Menus belong to the main thread.
    let _ = app.run_on_main_thread(move || {
        if let (Some(tray), Ok(menu)) = (app2.tray_by_id("main"), build_menu(&app2)) {
            let _ = tray.set_menu(Some(menu));
        }
    });
}

fn on_menu(app: &AppHandle, id: &str) {
    let floating = pet_win(app).is_some();
    match id {
        "open" => show_app(app),
        "float" if floating => stop_floating(app),
        "float" => {
            let auto = shared(app).auto_on;
            start_floating(app, "", auto);
        }
        "auto" => {
            let on = !shared(app).auto_on;
            if let Some(w) = move_target(app) {
                call(&w, "auto", &[json!(on)]);
            }
        }
        "ultra" => {
            let on = !shared(app).ultra_on;
            call_all(app, "ultra", &[json!(on)]);
        }
        "corner" => to_corner(app),
        "login" => {
            let on = !app.autolaunch().is_enabled().unwrap_or(false);
            set_start_at_login(app, on);
        }
        "claude" => toggle_claude_hooks(app),
        "update" => {
            if shared(app).update.state == "ready" {
                install_update(app, true);
            } else {
                check_updates(app, true);
            }
        }
        "quit" => app.exit(0),
        _ => {
            if let Some(name) = id.strip_prefix("play:").filter(|n| valid_move(n)) {
                if let Some(w) = move_target(app) {
                    call(&w, "play", &[json!(name)]);
                }
            } else if let Some(size) = id.strip_prefix("size:").filter(|n| SIZES.iter().any(|(s, _)| s == n)) {
                set_size(app, size);
            }
        }
    }
    refresh_tray(app); // check items tick themselves when clicked; show the real state
}

// ---- Calls from the pages (bridge.js), by the names bridge.js uses ----

mod commands {
    use super::*;

    pub(crate) fn is_pet(w: &WebviewWindow) -> bool {
        w.label() == "pet"
    }

    #[tauri::command]
    pub fn move_by(window: WebviewWindow, dx: f64, dy: f64) {
        if !is_pet(&window) || !dx.is_finite() || !dy.is_finite() {
            return;
        }
        let (Ok(pos), Ok(scale)) = (window.outer_position(), window.scale_factor()) else { return };
        let _ = window.set_position(PhysicalPosition::new(
            pos.x + (dx * scale).round() as i32,
            pos.y + (dy * scale).round() as i32,
        ));
    }

    #[tauri::command]
    pub fn drag_end(app: AppHandle, window: WebviewWindow) {
        if is_pet(&window) {
            remember_position(&app);
        }
    }

    #[tauri::command]
    pub fn set_hit_area(app: AppHandle, window: WebviewWindow, rects: Vec<Rect>, holding: bool) {
        if is_pet(&window) {
            let mut s = shared(&app);
            s.hit = rects.into_iter().take(64).collect();
            s.holding = holding;
        }
    }

    #[tauri::command]
    pub fn show_menu(app: AppHandle, window: WebviewWindow) {
        if let Ok(menu) = build_menu(&app) {
            let _ = window.popup_menu(&menu);
        }
    }

    #[tauri::command]
    pub fn start_floating(app: AppHandle, first: String, auto: bool) {
        super::start_floating(&app, &first, auto);
    }

    #[tauri::command]
    pub fn stop_floating(app: AppHandle) {
        super::stop_floating(&app);
    }

    #[tauri::command]
    pub fn open_app(app: AppHandle) {
        show_app(&app);
    }

    #[tauri::command]
    pub fn quit(app: AppHandle) {
        app.exit(0);
    }

    #[tauri::command]
    pub fn report_auto(app: AppHandle, on: bool) {
        shared(&app).auto_on = on;
        refresh_tray(&app);
    }

    #[tauri::command]
    pub fn report_ultra(app: AppHandle, on: bool) {
        shared(&app).ultra_on = on;
        refresh_tray(&app);
    }

    /// The app window and floating Clawd stay matched: each one's moves, Auto-play switch and work go to the other.
    #[tauri::command]
    pub fn sync(app: AppHandle, window: WebviewWindow, message: Value) {
        let mut m = serde_json::Map::new();
        if let Some(name) = message.get("move").and_then(Value::as_str).filter(|n| valid_move(n)) {
            m.insert("move".into(), json!(name));
        }
        for key in ["auto", "work"] {
            if let Some(on) = message.get(key).and_then(Value::as_bool) {
                m.insert(key.into(), json!(on));
            }
        }
        let other = if is_pet(&window) { app_win(&app) } else { pet_win(&app) };
        if let (Some(other), false) = (other, m.is_empty()) {
            call(&other, "sync", &[Value::Object(m)]);
        }
    }

    #[tauri::command]
    pub fn check_updates(app: AppHandle) {
        super::check_updates(&app, false);
    }

    #[tauri::command]
    pub fn install_update(app: AppHandle) {
        let state = shared(&app).update.state;
        match state {
            "ready" => super::install_update(&app, true),
            "portable" => open_releases(),
            _ => {}
        }
    }

    /// A page has loaded: what it needs to know.
    #[tauri::command]
    pub fn page_ready(app: AppHandle, window: WebviewWindow) {
        let (update, auto_on) = {
            let s = shared(&app);
            (s.update.clone(), s.auto_on)
        };
        call(&window, "floating", &[json!(pet_win(&app).is_some())]);
        if !update.state.is_empty() {
            call(&window, "update", &[serde_json::to_value(&update).unwrap_or_default()]);
        }
        // An app window opened while he floats takes his Auto-play switch.
        if !is_pet(&window) && pet_win(&app).is_some() {
            call(&window, "sync", &[json!({ "auto": auto_on })]);
        }
    }
}

fn main() {
    // Wayland doesn't let a window put itself in a corner or stay on top; through XWayland floating Clawd can.
    #[cfg(target_os = "linux")]
    if std::env::var_os("WAYLAND_DISPLAY").is_some() && std::env::var_os("GDK_BACKEND").is_none() {
        std::env::set_var("GDK_BACKEND", "x11");
    }
    let app = tauri::Builder::default()
        // Opening Clawd again (Start menu, Launchpad, a shortcut) brings up the app window, as on Android.
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            if !argv.iter().any(|a| a == AUTOSTART) {
                show_app(app);
            }
        }))
        .plugin(tauri_plugin_autostart::Builder::new().args([AUTOSTART]).app_name("Clawd").build())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::move_by,
            commands::drag_end,
            commands::set_hit_area,
            commands::show_menu,
            commands::start_floating,
            commands::stop_floating,
            commands::open_app,
            commands::quit,
            commands::report_auto,
            commands::report_ultra,
            commands::sync,
            commands::check_updates,
            commands::install_update,
            commands::page_ready
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            let config_dir = app.path().app_config_dir()?;
            // The Electron version kept its settings in <config>/Clawd.
            let legacy = app.path().config_dir().ok().map(|d| d.join("Clawd"));
            let settings = Settings::load(&config_dir, legacy.as_deref());
            app.manage(Clawd(Mutex::new(Shared {
                settings,
                config_dir,
                auto_on: false,
                ultra_on: false,
                hit: Vec::new(),
                holding: false,
                ignoring: true,
                update: Status::default(),
                install: Install::Dev,
                update_file: None,
                last_check: None,
                claude_hooks: claude_hooks_installed(),
                working_since: None,
            })));
            save_settings(&handle); // carries the Electron version's settings over for good
            init_start_at_login(&handle);

            // The tray icon; some Linux desktops have no tray, and Clawd still works from his right-click menu.
            let menu = build_menu(&handle)?;
            let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/tray.png"))?;
            let tray = TrayIconBuilder::with_id("main")
                .icon(icon)
                .tooltip("Clawd")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left, button_state: MouseButtonState::Up, ..
                    } = event
                    {
                        show_app(tray.app_handle());
                    }
                })
                .build(app);
            if let Err(err) = tray {
                eprintln!("Clawd: no tray icon: {err}");
            }
            app.on_menu_event(|app, event| on_menu(app, event.id().as_ref()));

            let react = handle.clone();
            claude::serve(move |kind| {
                let app = react.clone();
                let _ = react.run_on_main_thread(move || claude_event(&app, kind));
            });

            if std::env::args().any(|a| a == AUTOSTART) {
                start_floating(&handle, "", false);
            } else {
                show_app(&handle);
            }
            init_updates(&handle);
            watch_pointer(handle);
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("Clawd couldn't start");

    app.run(|app, event| {
        // Once neither window is open, Clawd quits (as on Android); a downloaded update installs on the way out.
        if let RunEvent::Exit = event {
            save_settings(app);
            install_update(app, false);
        }
    });
}
