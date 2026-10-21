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

/** Clawd fullscreen, with a button that sends him off to float over other apps. */
public class MainActivity extends Activity {
    private WebView web;
    private boolean bubbleAfterPermission = false;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
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
        startForegroundService(new Intent(this, BubbleService.class));
        moveTaskToBack(true);
    }

    @Override
    protected void onResume() {
        super.onResume();
        web.onResume();
        if (bubbleAfterPermission && Settings.canDrawOverlays(this)) {
            bubbleAfterPermission = false;
            startBubble();
        }
    }

    @Override
    protected void onPause() {
        web.onPause();
        super.onPause();
    }

    @Override
    protected void onDestroy() {
        web.destroy();
        super.onDestroy();
    }

    /** Calls the page can make (window.ClawdNative). They arrive on a background thread. */
    private class Bridge {
        @JavascriptInterface
        public void startBubble() {
            runOnUiThread(MainActivity.this::startBubble);
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
    }
}
