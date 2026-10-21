package com.dekelelely.clawd;

import android.Manifest;
import android.app.Activity;
import android.content.Intent;
import android.content.pm.PackageManager;
import android.graphics.Color;
import android.net.Uri;
import android.os.Build;
import android.os.Bundle;
import android.provider.Settings;
import android.view.View;
import android.view.WindowInsets;
import android.view.WindowInsetsController;
import android.webkit.JavascriptInterface;
import android.webkit.WebSettings;
import android.webkit.WebView;
import android.widget.Toast;

import org.json.JSONObject;

import java.lang.ref.WeakReference;

/** Clawd fullscreen, with a button that sends him off to float over other apps. */
public class MainActivity extends Activity {
    private static WeakReference<MainActivity> current = new WeakReference<>(null);
    private WebView web;
    private boolean bubbleAfterPermission = false;
    private String firstMove = "";   // the move the person last picked in the app, played first by the bubble
    private boolean floatAuto = false;   // whether the app was on Auto-play when the bubble was asked for
    private Updater updater;
    private boolean installAfterPermission = false;
    private boolean resumed = false;
    private Intent pendingConfirm = null;   // Android's "Update this app?" screen, waiting for the app to be in front

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        current = new WeakReference<>(this);
        updater = new Updater(this, this::showUpdateState);
        web = new WebView(this);
        WebSettings settings = web.getSettings();
        settings.setJavaScriptEnabled(true);
        settings.setDomStorageEnabled(true);
        web.setBackgroundColor(Color.rgb(13, 15, 18));
        web.addJavascriptInterface(new Bridge(), "ClawdNative");
        web.loadUrl("file:///android_asset/www/index.html#mode=full&platform=android");
        setContentView(web);
        hideSystemBars();

        if (Build.VERSION.SDK_INT >= 33
                && checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED) {
            requestPermissions(new String[] {Manifest.permission.POST_NOTIFICATIONS}, 1);
        }
    }

    private void hideSystemBars() {
        if (Build.VERSION.SDK_INT >= 30) {
            getWindow().setDecorFitsSystemWindows(false);
            WindowInsetsController controller = getWindow().getInsetsController();
            if (controller != null) {
                controller.hide(WindowInsets.Type.statusBars() | WindowInsets.Type.navigationBars());
                controller.setSystemBarsBehavior(WindowInsetsController.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE);
            }
        } else {
            getWindow().getDecorView().setSystemUiVisibility(
                    View.SYSTEM_UI_FLAG_IMMERSIVE_STICKY
                            | View.SYSTEM_UI_FLAG_FULLSCREEN
                            | View.SYSTEM_UI_FLAG_HIDE_NAVIGATION
                            | View.SYSTEM_UI_FLAG_LAYOUT_STABLE
                            | View.SYSTEM_UI_FLAG_LAYOUT_HIDE_NAVIGATION
                            | View.SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN);
        }
    }

    @Override
    public void onWindowFocusChanged(boolean hasFocus) {
        super.onWindowFocusChanged(hasFocus);
        if (hasFocus) hideSystemBars();
    }

    /** Starts the bubble, or first sends the person to the "Display over other apps" setting. */
    private void startBubble() {
        if (!Settings.canDrawOverlays(this)) {
            bubbleAfterPermission = true;
            Toast.makeText(this, "Allow Clawd to display over other apps, then come back", Toast.LENGTH_LONG).show();
            startActivity(new Intent(Settings.ACTION_MANAGE_OVERLAY_PERMISSION, Uri.parse("package:" + getPackageName())));
            return;
        }
        startForegroundService(new Intent(this, BubbleService.class)
                .putExtra(BubbleService.EXTRA_FIRST_MOVE, firstMove)
                .putExtra(BubbleService.EXTRA_AUTO, floatAuto));
        moveTaskToBack(true);
    }

    /** Installs the new version, first sending the person to "Install unknown apps" if Clawd may not install yet. */
    private void installUpdate() {
        if (!getPackageManager().canRequestPackageInstalls()) {
            installAfterPermission = true;
            Toast.makeText(this, "Allow Clawd to install updates, then come back", Toast.LENGTH_LONG).show();
            startActivity(new Intent(Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES, Uri.parse("package:" + getPackageName())));
            return;
        }
        updater.install();
    }

    private void showUpdateState(String json) {
        if (isDestroyed()) return;
        web.evaluateJavascript("window.clawdUpdate && window.clawdUpdate(" + JSONObject.quote(json) + ")", null);
    }

    /** A message from the bubble for the app's page to match (see BubbleService.relay). Main thread. */
    static void deliver(String json) {
        MainActivity activity = current.get();
        if (activity == null || activity.isDestroyed()) return;
        activity.web.evaluateJavascript("window.clawdSync && window.clawdSync(" + JSONObject.quote(json) + ")", null);
    }

    /** Android's answer about an update install (from UpdateReceiver). Main thread. */
    static void updateStatus(Intent intent) {
        MainActivity activity = current.get();
        if (activity == null) return;
        Intent confirm = activity.updater.onStatus(intent);
        if (confirm == null) return;
        if (activity.resumed) activity.startActivity(confirm);
        else activity.pendingConfirm = confirm;
    }

    @Override
    protected void onResume() {
        super.onResume();
        resumed = true;
        web.onResume();
        updater.refresh();   // on opening and each time the app comes back; the update button shows the result
        if (bubbleAfterPermission && Settings.canDrawOverlays(this)) {
            bubbleAfterPermission = false;
            startBubble();
        }
        if (installAfterPermission && getPackageManager().canRequestPackageInstalls()) {
            installAfterPermission = false;
            updater.install();
        }
        if (pendingConfirm != null) {
            startActivity(pendingConfirm);
            pendingConfirm = null;
        }
    }

    @Override
    protected void onPause() {
        resumed = false;
        web.onPause();
        super.onPause();
    }

    /** Closes the app's screen if it is open (the bubble's triple-tap). Call on the main thread. */
    static void closeIfOpen() {
        MainActivity activity = current.get();
        if (activity != null) activity.finishAndRemoveTask();
    }

    @Override
    protected void onDestroy() {
        if (current.get() == this) current.clear();
        web.destroy();
        super.onDestroy();
    }

    /** Calls the page can make (window.ClawdNative). They arrive on a background thread. */
    private class Bridge {
        /** first: a move to open with, or "" for a random one. auto: whether the app is on Auto-play. */
        @JavascriptInterface
        public void startBubble(String first, boolean auto) {
            runOnUiThread(() -> {
                firstMove = first == null ? "" : first;
                floatAuto = auto;
                MainActivity.this.startBubble();
            });
        }

        @JavascriptInterface
        public boolean touchThrough() {
            return BubbleService.touchThrough(MainActivity.this);
        }

        /** Taps pass through the space around the bubble (Clawd at 80% opacity on Android 12+), or not (full colour). */
        @JavascriptInterface
        public void setTouchThrough(boolean on) {
            runOnUiThread(() -> {
                BubbleService.setTouchThrough(MainActivity.this, on);
                if (BubbleService.running) {
                    startService(new Intent(MainActivity.this, BubbleService.class).setAction(BubbleService.ACTION_SETTINGS));
                }
            });
        }

        @JavascriptInterface
        public void stopBubble() {
            runOnUiThread(() -> stopService(new Intent(MainActivity.this, BubbleService.class)));
        }

        @JavascriptInterface
        public boolean bubbleRunning() {
            return BubbleService.running;
        }

        @JavascriptInterface
        public void openApp() {
            // Already open.
        }

        @JavascriptInterface
        public void setHitRect(float x, float y, float w, float h) {
            // Only the bubble has a touch pad.
        }

        /** A move the app started, or its Auto-play switch, for the bubble to match (see BubbleService.relay). */
        @JavascriptInterface
        public void sync(String json) {
            runOnUiThread(() -> {
                String message = BubbleService.relay(json);
                if (message != null) BubbleService.deliver(message);
            });
        }

        /** The Auto-play switch the app and the bubble share, for an app screen that opens while he floats. */
        @JavascriptInterface
        public boolean linkedAuto() {
            return BubbleService.linkedAuto;
        }

        /** The update button's state, as JSON (see Updater.stateJson). */
        @JavascriptInterface
        public String updateState() {
            return updater.stateJson();
        }

        @JavascriptInterface
        public void checkUpdates() {
            runOnUiThread(() -> updater.check());
        }

        @JavascriptInterface
        public void installUpdate() {
            runOnUiThread(() -> MainActivity.this.installUpdate());
        }
    }
}
