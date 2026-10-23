# Voice to WhatsApp

A home-screen mic widget for Android. Tap it, speak (Hebrew or English), check the text, and it opens
your own WhatsApp chat with yourself with the text already typed in. You tap Send in WhatsApp.

- Kotlin, minSdk 26 (Android 8.0), targetSdk 35. Uses only the Android framework: no AndroidX,
  no SDKs, no API keys, no permissions.
- The app's only network action is handing a `https://wa.me/<your number>?text=…` link to WhatsApp.
  Speech recognition is done by the phone's recognizer app (normally the Google app), which may use
  Google's servers. The app doesn't set `EXTRA_PREFER_OFFLINE`, so the recognizer decides.
- Auto Backup is off (`allowBackup="false"`), so your number never goes to a cloud backup. After a
  reinstall you enter it again.

## How it works

| Piece | File |
|---|---|
| Widget: one big tap target, resizable from 1 cell to a full page | `MicWidgetProvider.kt`, `res/xml/mic_widget_info.xml`, `res/layout/widget_mic.xml` |
| Transparent activity that starts the recognizer at once | `VoiceActivity.kt` |
| Review screen: edit text, עברית / English toggle, Dictate again, Send | `ReviewActivity.kt` |
| First-run setup and Settings (same screen) | `SetupActivity.kt` |
| Recognizer intent and result/error handling | `Speech.kt` |
| `wa.me` link and opening WhatsApp / WhatsApp Business | `WhatsApp.kt` |
| Number clean-up and warnings | `PhoneNumber.kt` |

## Build and install

You need a JDK 17 or 21 and the Android SDK (API 35). Android Studio provides both.

### Option A: Android Studio (easiest)

1. Install Android Studio from https://developer.android.com/studio.
2. **File → Open** and pick this `voice-to-whatsapp` folder (not the repository root).
3. Wait for the Gradle sync. If Studio asks to install "Android SDK Platform 35" or accept licenses,
   accept. If it complains about the Gradle JDK, go to **Settings → Build, Execution, Deployment →
   Build Tools → Gradle → Gradle JDK** and pick a JDK 17 or 21 (the bundled `jbr-21` works).
   If it offers an Android Gradle Plugin upgrade, you can accept or skip it.
4. On the phone, turn on USB debugging: **Settings → About phone →** tap **Build number** seven
   times, then **Settings → System → Developer options → USB debugging** (menu names vary a little
   by manufacturer). Plug the phone in and accept the "Allow USB debugging?" prompt.
5. Pick your phone in the device drop-down and press **Run ▶**.
   To get an APK file instead: **Build → Build App Bundle(s) / APK(s) → Build APK(s)**. It lands in
   `app/build/outputs/apk/debug/app-debug.apk`.

### Option B: command line with adb

```sh
# One-time SDK setup, if you don't have Android Studio
# (command-line tools: https://developer.android.com/studio#command-line-tools-only)
sdkmanager "platform-tools" "platforms;android-35" "build-tools;35.0.0"
sdkmanager --licenses

# Tell Gradle where the SDK is (or put sdk.dir=/path/to/sdk in local.properties)
export ANDROID_HOME="$HOME/Android/Sdk"        # macOS default: $HOME/Library/Android/sdk

cd voice-to-whatsapp
./gradlew assembleDebug                         # Windows: gradlew.bat assembleDebug
./gradlew test                                  # optional: unit tests

adb devices                                     # phone must show as "device", not "unauthorized"
adb install -r app/build/outputs/apk/debug/app-debug.apk
```

Without a cable you can copy `app-debug.apk` to the phone and open it from a file manager. Allow
"Install unknown apps" for that file manager when Android asks.

## First use

1. Open **Voice to WhatsApp** and enter your own number in international format, e.g.
   `+972 50 123 4567` (saved as `972501234567`).
2. Tap **Add widget to home screen**, or long-press the home screen → Widgets → *Dictate to WhatsApp*.
   Resize it as you like.
3. Tap the widget, speak, review, **Send to WhatsApp**, then tap Send in WhatsApp.

If speech recognition reports "not available", install or enable the Google app. For Hebrew,
make sure Hebrew is available under the Google app's voice settings.
