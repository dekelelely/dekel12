package com.dekelelely.clawd;

import android.annotation.SuppressLint;
import android.app.Notification;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.app.PendingIntent;
import android.app.Service;
import android.content.Context;
import android.content.Intent;
import android.content.pm.ServiceInfo;
import android.graphics.Color;
import android.graphics.PixelFormat;
import android.graphics.drawable.Icon;
import android.hardware.input.InputManager;
import android.os.Build;
import android.os.Handler;
import android.os.IBinder;
import android.os.Looper;
import android.provider.Settings;
import android.util.DisplayMetrics;
import android.view.Gravity;
import android.view.MotionEvent;
import android.view.View;
import android.view.ViewConfiguration;
import android.view.WindowManager;
import android.webkit.JavascriptInterface;
import android.webkit.WebSettings;
import android.webkit.WebView;

import org.json.JSONException;
import org.json.JSONObject;

/**
 * Clawd floating over other apps. Two windows: a large one that draws him, with room for his jumps and travel, and a
 * small invisible touch pad that the page keeps over his body. With "taps through" on, the large window lets touches
 * through to the apps below, which Android 12+ only allows at 80% opacity; with it off (the default) he is drawn in
 * full colour and the large window takes the touches around him.
 * Tap him for a move, double-tap to open the app and hide the bubble, triple-tap to close the app and the bubble,
 * long-press to open the app, drag to move him.
 */
public class BubbleService extends Service {
    static volatile boolean running = false;
    private static BubbleService instance;   // main thread only
    static volatile boolean linkedAuto = false;   // the Auto-play switch the app and the bubble share
    private static final String CHANNEL = "clawd";
    private static final String ACTION_STOP = "com.dekelelely.clawd.STOP";
    static final String ACTION_SETTINGS = "com.dekelelely.clawd.SETTINGS";
    static final String EXTRA_FIRST_MOVE = "firstMove";
    static final String EXTRA_AUTO = "auto";
    private static final String PREFS = "clawd";
    private static final String PREF_TOUCH_THROUGH = "touchThrough";

    static boolean touchThrough(Context context) {
        return context.getSharedPreferences(PREFS, MODE_PRIVATE).getBoolean(PREF_TOUCH_THROUGH, false);
    }

    static void setTouchThrough(Context context, boolean on) {
        context.getSharedPreferences(PREFS, MODE_PRIVATE).edit().putBoolean(PREF_TOUCH_THROUGH, on).apply();
    }

    /** Move names are lowercase words joined by hyphens; anything else is ignored. */
    private static String moveName(String name) {
        return name != null && name.matches("[a-z]+(-[a-z]+)*") ? name : null;
    }

    private final Handler main = new Handler(Looper.getMainLooper());
    private WindowManager windowManager;
    private WebView web;
    private View pad;
    private WindowManager.LayoutParams stageParams;
    private WindowManager.LayoutParams padParams;
    private float density;
    private boolean dragging;
    private boolean holdPad;   // while a finger is down or taps are being counted, the pad stays put under it

    @Override
    public IBinder onBind(Intent intent) {
        return null;
    }

    @Override
    public int onStartCommand(Intent intent, int flags, int startId) {
        if (intent != null && ACTION_STOP.equals(intent.getAction())) {
            stopSelf();
            return START_NOT_STICKY;
        }
        if (intent != null && ACTION_SETTINGS.equals(intent.getAction())) {
            if (web != null) {
                applyTouchThrough();
                windowManager.updateViewLayout(web, stageParams);
            }
            return START_STICKY;
        }
        startInForeground();
        if (web == null && !Settings.canDrawOverlays(this)) {
            // Android restarting the bubble after "Display over other apps" was turned off: it can't be shown.
            stopSelf();
            return START_NOT_STICKY;
        }
        String first = moveName(intent == null ? null : intent.getStringExtra(EXTRA_FIRST_MOVE));
        if (web == null) {
            linkedAuto = intent != null && intent.getBooleanExtra(EXTRA_AUTO, false);
            showBubble(first);
        } else if (first != null) {
            web.evaluateJavascript("window.clawdPet && window.clawdPet.play('" + first + "')", null);
        }
        return START_STICKY;
    }

    private void startInForeground() {
        NotificationManager manager = getSystemService(NotificationManager.class);
        manager.createNotificationChannel(new NotificationChannel(CHANNEL, "Clawd bubble", NotificationManager.IMPORTANCE_MIN));
        PendingIntent open = PendingIntent.getActivity(this, 1,
                new Intent(this, MainActivity.class), PendingIntent.FLAG_IMMUTABLE);
        PendingIntent hide = PendingIntent.getService(this, 2,
                new Intent(this, BubbleService.class).setAction(ACTION_STOP), PendingIntent.FLAG_IMMUTABLE);
        Notification notification = new Notification.Builder(this, CHANNEL)
                .setSmallIcon(R.drawable.ic_stat_clawd)
                .setContentTitle("Clawd is on your screen")
                .setContentText("Tap to open him fullscreen")
                .setContentIntent(open)
                .addAction(new Notification.Action.Builder((Icon) null, "Hide Clawd", hide).build())
                .setOngoing(true)
                .build();
        if (Build.VERSION.SDK_INT >= 34) {
            startForeground(1, notification, ServiceInfo.FOREGROUND_SERVICE_TYPE_SPECIAL_USE);
        } else {
            startForeground(1, notification);
        }
    }

    private static WindowManager.LayoutParams overlay(int width, int height, int flags) {
        WindowManager.LayoutParams params = new WindowManager.LayoutParams(
                width, height, WindowManager.LayoutParams.TYPE_APPLICATION_OVERLAY, flags, PixelFormat.TRANSLUCENT);
        params.gravity = Gravity.TOP | Gravity.START;
        return params;
    }

    @SuppressLint({"SetJavaScriptEnabled", "ClickableViewAccessibility"})
    private void showBubble(String first) {
        running = true;
        instance = this;
        windowManager = (WindowManager) getSystemService(WINDOW_SERVICE);
        DisplayMetrics metrics = getResources().getDisplayMetrics();
        density = metrics.density;

        // The stage: wide and short (the same shape as the page's pet camera), touches pass through it.
        int width = Math.round(214 * density);
        int height = Math.round(139 * density);
        stageParams = overlay(width, height, 0);
        stageParams.x = metrics.widthPixels - width;
        stageParams.y = metrics.heightPixels - height - Math.round(110 * density);
        applyTouchThrough();

        web = new WebView(this);
        web.setBackgroundColor(Color.TRANSPARENT);
        web.setLayerType(View.LAYER_TYPE_HARDWARE, null);
        WebSettings settings = web.getSettings();
        settings.setJavaScriptEnabled(true);
        settings.setDomStorageEnabled(true);
        web.addJavascriptInterface(new Bridge(), "ClawdNative");
        web.loadUrl("file:///android_asset/www/index.html#mode=pet&platform=android"
                + (first == null ? "" : "&first=" + first) + (linkedAuto ? "&auto=1" : ""));
        windowManager.addView(web, stageParams);

        // The touch pad starts where he stands (bottom middle of the stage); the page then keeps it on him.
        int size = Math.round(80 * density);
        padParams = overlay(size, size, WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE);
        padParams.x = stageParams.x + (width - size) / 2;
        padParams.y = stageParams.y + height - size;
        pad = new View(this);
        pad.setOnTouchListener(new PadTouch());
        windowManager.addView(pad, padParams);
    }

    /** Sets the stage window's flags and opacity from the "taps through" setting. */
    private void applyTouchThrough() {
        int flags = WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE | WindowManager.LayoutParams.FLAG_HARDWARE_ACCELERATED;
        stageParams.alpha = 1f;
        if (touchThrough(this)) {
            flags |= WindowManager.LayoutParams.FLAG_NOT_TOUCHABLE;
            if (Build.VERSION.SDK_INT >= 31) {
                // Android 12+ only lets touches through an overlay from another app at or below this opacity.
                InputManager input = getSystemService(InputManager.class);
                stageParams.alpha = input.getMaximumObscuringOpacityForTouch();
            }
        }
        stageParams.flags = flags;
    }

    private void openApp() {
        Intent intent = new Intent(this, MainActivity.class);
        intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
        startActivity(intent);
    }

    /**
     * The app and the bubble stay matched: a message from one page (a move it started, its Auto-play switch) is played
     * by the other. Called on the main thread with the message as JSON; the Auto-play switch is remembered for an app
     * screen that opens later.
     */
    static String relay(String json) {
        try {
            JSONObject in = new JSONObject(json);
            JSONObject out = new JSONObject();
            String move = moveName(in.optString("move", null));
            if (move != null) out.put("move", move);
            if (in.has("auto")) {
                linkedAuto = in.optBoolean("auto");
                out.put("auto", linkedAuto);
            }
            return out.length() > 0 ? out.toString() : null;
        } catch (JSONException e) {
            return null;
        }
    }

    static void deliver(String json) {
        if (instance != null && instance.web != null) {
            instance.web.evaluateJavascript("window.clawdSync && window.clawdSync(" + JSONObject.quote(json) + ")", null);
        }
    }

    /** Triple-tap: close the app and the bubble. */
    private void closeEverything() {
        MainActivity.closeIfOpen();
        stopSelf();
    }

    /**
     * Touches on Clawd: a tap plays a move straight away; taps are then counted for a moment, and two mean "open the
     * app and hide the bubble", three mean "close the app and the bubble". Meanwhile the pad stays where it is, though
     * the move the first tap started may carry Clawd away, so the next taps land on it. A long press opens the app; a
     * drag moves both windows, using raw screen coordinates so moving the pad under the finger doesn't feed back into
     * the drag.
     */
    private class PadTouch implements View.OnTouchListener {
        private static final int MULTI_TAP_MS = 400;   // the most time between taps of a double or triple tap
        private final int slop = ViewConfiguration.get(BubbleService.this).getScaledTouchSlop();
        private float downX, downY;
        private int stageX, stageY, padX, padY;
        private boolean longPressed;
        private int taps;
        private final Runnable longPress = () -> {
            longPressed = true;
            taps = 0;
            openApp();
        };
        private final Runnable settleTaps = () -> {
            holdPad = false;
            if (taps == 2) {
                openApp();
                stopSelf();
            } else if (taps >= 3) {
                closeEverything();
            }
            taps = 0;
        };

        @Override
        public boolean onTouch(View view, MotionEvent event) {
            switch (event.getActionMasked()) {
                case MotionEvent.ACTION_DOWN:
                    downX = event.getRawX();
                    downY = event.getRawY();
                    stageX = stageParams.x;
                    stageY = stageParams.y;
                    padX = padParams.x;
                    padY = padParams.y;
                    dragging = false;
                    longPressed = false;
                    holdPad = true;
                    main.removeCallbacks(settleTaps);
                    main.postDelayed(longPress, ViewConfiguration.getLongPressTimeout());
                    return true;
                case MotionEvent.ACTION_MOVE: {
                    int dx = Math.round(event.getRawX() - downX);
                    int dy = Math.round(event.getRawY() - downY);
                    if (!dragging && Math.hypot(dx, dy) > slop * 1.5) {
                        dragging = true;
                        taps = 0;
                        main.removeCallbacks(longPress);
                    }
                    if (dragging) {
                        stageParams.x = stageX + dx;
                        stageParams.y = stageY + dy;
                        padParams.x = padX + dx;
                        padParams.y = padY + dy;
                        windowManager.updateViewLayout(web, stageParams);
                        windowManager.updateViewLayout(pad, padParams);
                    }
                    return true;
                }
                case MotionEvent.ACTION_UP:
                    main.removeCallbacks(longPress);
                    if (dragging || longPressed) {
                        dragging = false;
                        holdPad = false;
                        return true;
                    }
                    taps++;
                    if (taps == 1) web.evaluateJavascript("window.clawdPet && window.clawdPet.tap()", null);
                    main.postDelayed(settleTaps, MULTI_TAP_MS);
                    return true;
                case MotionEvent.ACTION_CANCEL:
                    main.removeCallbacks(longPress);
                    dragging = false;
                    if (taps == 0) holdPad = false;
                    return true;
                default:
                    return true;
            }
        }
    }

    /** Calls the bubble's page can make (window.ClawdNative). They arrive on a background thread. */
    private class Bridge {
        @JavascriptInterface
        public void openApp() {
            BubbleService.this.openApp();
        }

        @JavascriptInterface
        public void stopBubble() {
            main.post(BubbleService.this::stopSelf);
        }

        @JavascriptInterface
        public void startBubble(String first, boolean auto) {
            // Already floating.
        }

        /** A move the bubble started, or its Auto-play switch, for the app to match (see relay). */
        @JavascriptInterface
        public void sync(String json) {
            main.post(() -> {
                String message = relay(json);
                if (message != null) MainActivity.deliver(message);
            });
        }

        @JavascriptInterface
        public boolean touchThrough() {
            return BubbleService.touchThrough(BubbleService.this);
        }

        @JavascriptInterface
        public void setTouchThrough(boolean on) {
            // Set from the app.
        }

        @JavascriptInterface
        public boolean bubbleRunning() {
            return true;
        }

        /** Triple tap that landed on the stage rather than the pad: close the app and the bubble. */
        @JavascriptInterface
        public void closeAll() {
            main.post(BubbleService.this::closeEverything);
        }

        /** Where Clawd's body is in the page, in CSS pixels; the touch pad moves over it. */
        @JavascriptInterface
        public void setHitRect(float x, float y, float w, float h) {
            main.post(() -> {
                if (pad == null || dragging || holdPad) return;
                int margin = Math.round(10 * density);
                int min = Math.round(56 * density);
                int width = Math.max(min, Math.round(w * density) + 2 * margin);
                int height = Math.max(min, Math.round(h * density) + 2 * margin);
                int x0 = stageParams.x + Math.round((x + w / 2) * density) - width / 2;
                int y0 = stageParams.y + Math.round((y + h / 2) * density) - height / 2;
                if (Math.abs(x0 - padParams.x) < 3 && Math.abs(y0 - padParams.y) < 3
                        && Math.abs(width - padParams.width) < 3 && Math.abs(height - padParams.height) < 3) {
                    return;
                }
                padParams.x = x0;
                padParams.y = y0;
                padParams.width = width;
                padParams.height = height;
                windowManager.updateViewLayout(pad, padParams);
            });
        }
    }

    @Override
    public void onDestroy() {
        running = false;
        if (instance == this) instance = null;
        if (pad != null) {
            windowManager.removeView(pad);
            pad = null;
        }
        if (web != null) {
            windowManager.removeView(web);
            web.destroy();
            web = null;
        }
        super.onDestroy();
    }
}
