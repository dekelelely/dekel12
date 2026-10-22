# Clawd apps

Clawd sits in the corner of your screen while you work, doing the same moves as on Claude Code's welcome screen, plus a few of his own: he waves, dances, naps, does jumping jacks, takes a bow, moonwalks and thinks things over. One page (`web/index.html`) and one scene (`../clawd-engine.js`) power the web page, the desktop app (Windows, macOS and Linux) and the Android app. The Nintendo Switch app plays the same moves with the Rust port of the scene (`../rust`).

## Getting the apps

Every push that touches `clawd-3d/` builds every app on GitHub (workflow `.github/workflows/clawd-apps.yml`) and publishes them as a release, version 1.*N* where *N* is the build number. Download from the repository's **Releases** page (the latest one), no GitHub login needed:

- **Windows**: `Clawd-Setup-1.N.0.exe` installs Clawd and keeps it up to date; `Clawd-Portable-1.N.0.exe` runs without installing but can't update itself.
- **macOS** (Apple silicon and Intel): `Clawd-1.N.0-macOS.dmg`. Open it and drag **Clawd** to **Applications**.
- **Linux**: `Clawd-1.N.0-x86_64.AppImage` (make it executable and run it; it updates itself) or `Clawd_1.N.0_amd64.deb`.
- **Android**: `Clawd.apk`. This link always downloads the newest one: <https://github.com/dekelelely/dekel12/releases/latest/download/Clawd.apk>
- **Nintendo Switch** (homebrew): `clawd.nro`. See [switch/README.md](switch/README.md).

Download from **Releases** only. The **Actions** tab lists every build ever made, so a download from there is easily an old version. The update button at the top right of the app shows which version you have (**Up to date · 1.N**).

The apps aren't code-signed, so the first run needs a yes: Windows SmartScreen asks first (**More info → Run anyway**); macOS says it can't check Clawd for malware (**System Settings → Privacy & Security → Open Anyway**, once); Android asks you to allow installs from your browser or file manager.

## Updates

You only download Clawd once. After that, each app checks for a new version when it opens and whenever you come back to it (on the desktop, also every hour while Clawd floats):

- **Android**: the button at the top right of the app says **Update to 1.N** when there is one. Tap it, and Android asks **Update this app?**; tap **Update**. The first time, Android first asks you to let Clawd install apps (**Allow from this source**). The button also shows **Up to date** or lets you check again.
- **Windows, macOS, Linux** (the installed app, the app in Applications, the AppImage): Clawd downloads a new version in the background. **Restart to update** in the tray or right-click menu (or the update button) puts it in place and starts him again; otherwise it goes in the next time Clawd quits. **Check for updates** in the menu checks right away. The portable .exe and the .deb open the Releases page instead.

The update check reads the public Releases page; it needs no account, key or paid service.

Coming from the Electron version of Clawd for Windows (1.22 and older): it updates to this one like any update. The installer removes the old app and its **Start with Windows** entry, and Clawd keeps your floating size, his spot and the **Start with Windows** setting.

## Windows, macOS and Linux

Clawd for the desktop works like the Android app. Open **Clawd** for the app window with every move. Click **Float on screen** and Clawd stands on the taskbar (or the Dock) in the bottom-right corner, always on top, with no window around him. If you picked a move in the app first, he opens with that one; otherwise with a random one. His invisible window is wide enough for his jumps and travel, and clicks pass through it to whatever is behind him except on Clawd himself. His eyes follow your mouse, and look ahead again once it rests.

On his own he loops: a random move, five seconds of rest after it ends, then the next. Clicking him takes over, and each click plays a new move. Once you stop, he finishes the move, waits five more seconds and goes back to looping. With **Auto-play** on he plays random moves back to back instead, with no rest.

The app window and floating Clawd stay matched: a move started in one plays in the other too, and they share the **Auto-play** switch. Floating Clawd picks the random moves (only moves that start where he stands, so none slide in from off the window).

| | |
|---|---|
| Click Clawd | A new random move right away, even in the middle of one |
| Drag Clawd | Move him anywhere; he remembers the spot |
| Double-click Clawd | Open the app window and put floating Clawd away |
| Triple-click Clawd | Close the app window and floating Clawd (quits Clawd) |
| Long-press Clawd | Open the app window, keeping him out |
| **Hide floating Clawd** in the app | Put him away |
| Right-click Clawd, or the tray icon | Open the app, float or hide him, play a move, auto-play, ultra effort, floating size (small / medium / large), move to corner, start at login, react to Claude Code, check for updates, quit |

Closing the app window while he floats keeps him out; the tray icon brings the app back. With **Start with Windows** (**Open at login** on macOS, **Start at login** on Linux) on, he starts floating when you log in, without the app window. Turn on **Ultra effort (electric guitar)** in the menu and he grabs an electric guitar and rocks out, then keeps jamming between moves. **Sound** in the app window plays the ultracode riff with his guitar routine (in the app window only, never from floating Clawd).

On Linux, floating needs a desktop that draws see-through windows (most do). Wayland doesn't let windows put themselves in a corner or stay on top, so there Clawd runs through XWayland (set `GDK_BACKEND=wayland` to run him natively, without floating on top).

### Claude Code

Clawd can follow what Claude Code is doing: he waves when you start a session, types on his laptop while Claude works, celebrates when it's done, and jumps to get your attention when it needs you (a permission prompt, or waiting for your next message). Turn on **React to Claude Code** in the menu: Clawd asks first, then adds one hook to Claude Code's settings (`~/.claude/settings.json`, keeping a copy of the file as `settings.json.clawd-backup`). Turning it off takes the hook out again.

The hook runs `curl -s -m 2 --data-binary @- http://127.0.0.1:47823/claude || true` for these events: `SessionStart` (hello, for new sessions only), `UserPromptSubmit`, `PreToolUse` (working), `Stop` (done), `Notification` (needs you) and `SessionEnd`. It only talks to Clawd on your own computer, never waits more than two seconds, and does nothing when Clawd isn't running. To add it by hand, put the same command under those events in your settings.

## Android

Open **Clawd** for the fullscreen view with every move. Tap **Float on screen**, allow **Display over other apps** when Android asks, and come back: Clawd now floats over your other apps. If you picked a move in the app first, he opens with that one; otherwise with a random one. After that he loops on his own: a random move, five seconds of rest after it ends, then the next. Tapping him takes over, and each tap plays a new move right away. Once you stop, he finishes the move, waits five more seconds and goes back to looping. If **Auto-play** was on in the app, he plays random moves back to back instead.

The app and the bubble stay matched, as on the desktop: a move started in one plays in the other, and they share the **Auto-play** switch.

**Taps through** in the app decides what happens to taps on the space just around him. Off (the default): he is drawn in full colour, and that space takes your taps. On: taps there go to the app below, but on Android 12 and newer he is drawn at 80% opacity, the most Android allows for an overlay that lets touches through. Either way, taps on Clawd himself are his.

| | |
|---|---|
| Tap Clawd | A new random move right away, even in the middle of one |
| Drag Clawd | Move the bubble |
| Double-tap Clawd | Open the fullscreen app and hide the bubble |
| Triple-tap Clawd | Close the app and the bubble |
| Long-press Clawd | Open the fullscreen app, keeping the bubble |
| Notification → **Hide Clawd**, or **Hide bubble** in the app | Remove the bubble |

The **ultracode** button plays the electric-guitar routine once, and **Sound** plays its riff with it.

Builds made after October 21, 2026 are all signed with the same key (`android/app/debug.keystore`), so a new APK installs over the previous one and the update button works. If Android says "App not installed" when updating from an older build, uninstall Clawd once and install again.

## Building them yourself

```sh
cd clawd-3d/apps
npm install
npm run prepare-web        # copies index.html, clawd-engine.js, the riff and three.js into the apps

cd desktop                 # Windows, macOS, Linux (needs Node.js 20 and Rust: https://rustup.rs)
npm install                # Linux also needs WebKitGTK: see https://tauri.app/start/prerequisites/
npm start                  # try it
npm run dist               # installer, app or AppImage in desktop/target/release/bundle (version 1.0.0, so any release counts as newer)

cd ../android              # Android (needs JDK 17 and the Android SDK, e.g. from Android Studio)
gradle assembleDebug       # Gradle 8.7 or newer; APK in app/build/outputs/apk/debug (version 1.1)
```

Edit the page in `web/` and the scene in `../clawd-engine.js`; the copies inside `desktop/web` and `android/.../assets/www` are generated. The desktop app is Rust (`desktop/src`): `main.rs` has the windows, tray and menu, `claude.rs` the Claude Code hook, `updates.rs` the updates, and `bridge.js` connects the page to it.

When you change a move in `../clawd-engine.js`, run `node clawd-3d/rust/tools/export-moves.mjs` so the Rust port (and the Switch app) plays it too; CI checks that.
