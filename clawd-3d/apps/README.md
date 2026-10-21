# Clawd apps

Clawd sits in the corner of your screen while you work, doing the same moves as on Claude Code's welcome screen. One page (`web/index.html`) and one scene (`../clawd-engine.js`) power the web page, the Windows app and the Android app.

## Getting the apps

Every push that touches `clawd-3d/` builds both apps on GitHub (workflow `.github/workflows/clawd-apps.yml`). Open the repository's **Actions** tab, pick the latest **Clawd apps** run, and download from **Artifacts**:

- **Clawd-Windows**: `Clawd-Setup-1.0.0.exe` installs Clawd; `Clawd-Portable-1.0.0.exe` runs without installing.
- **Clawd-Android**: `Clawd.apk`.

The apps aren't code-signed, so Windows SmartScreen asks before the first run (**More info → Run anyway**) and Android asks you to allow installs from your browser or file manager.

## Windows

Clawd stands on the taskbar in the bottom-right corner, always on top, with no window around him. His invisible window is wide enough for his jumps and travel, and clicks pass through it to whatever is behind him except on Clawd himself.

| | |
|---|---|
| Click Clawd | A new random move right away, even in the middle of one |
| Drag Clawd | Move him anywhere; he remembers the spot |
| Double-click Clawd | Fullscreen (Esc or **Back to corner** to return) |
| Right-click Clawd, or the tray icon | Play a move, auto-play, fullscreen, size (small / medium / large), move to corner, start with Windows, quit |

On his own he loops: a random move, five seconds of rest after it ends, then the next. Clicking him takes over, and each click plays a new move. Once you stop, he finishes the move, waits five more seconds and goes back to looping. Turn on **Ultra effort (electric guitar)** in the menu and he grabs an electric guitar and rocks out, then keeps jamming between moves.

## Android

Open **Clawd** for the fullscreen view with every move. Tap **Float on screen**, allow **Display over other apps** when Android asks, and come back: Clawd now floats over your other apps. If you picked a move in the app first, he opens with that one; otherwise with a random one. After that he loops on his own: a random move, five seconds of rest after it ends, then the next. Tapping him takes over, and each tap plays a new move right away. Once you stop, he finishes the move, waits five more seconds and goes back to looping.

**Taps through** in the app decides what happens to taps on the space just around him. Off (the default): he is drawn in full colour, and that space takes your taps. On: taps there go to the app below, but on Android 12 and newer he is drawn at 80% opacity, the most Android allows for an overlay that lets touches through. Either way, taps on Clawd himself are his.

| | |
|---|---|
| Tap Clawd | A new random move right away, even in the middle of one |
| Drag Clawd | Move the bubble |
| Double-tap Clawd | Open the fullscreen app and hide the bubble |
| Triple-tap Clawd | Close the app and the bubble |
| Long-press Clawd | Open the fullscreen app, keeping the bubble |
| Notification → **Hide Clawd**, or **Hide bubble** in the app | Remove the bubble |

The **ultracode** button plays the electric-guitar routine once.

Builds made after October 21, 2026 are all signed with the same key (`android/app/debug.keystore`), so a new APK installs over the previous one. If Android says "App not installed" when updating from an older build, uninstall Clawd once and install again.

## Building them yourself

```sh
cd clawd-3d/apps
npm install
npm run prepare-web        # copies index.html, clawd-engine.js and three.js into both apps

cd desktop                 # Windows (needs Node.js 20)
npm install
npm start                  # try it
npm run dist               # installer + portable .exe in desktop/dist

cd ../android              # Android (needs JDK 17 and the Android SDK, e.g. from Android Studio)
gradle assembleDebug       # Gradle 8.7 or newer; APK in app/build/outputs/apk/debug
```

Edit the page in `web/` and the scene in `../clawd-engine.js`; the copies inside `desktop/web` and `android/.../assets/www` are generated.
