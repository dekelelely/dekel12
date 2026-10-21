package com.dekelelely.clawd;

import android.annotation.SuppressLint;
import android.app.Notification;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.app.PendingIntent;
import android.app.Service;
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
import android.util.DisplayMetrics;
import android.view.GestureDetector;
import android.view.Gravity;
import android.view.MotionEvent;
import android.view.View;
import android.view.ViewConfiguration;
import android.view.WindowManager;
import android.webkit.JavascriptInterface;
import android.webkit.WebSettings;
import android.webkit.WebView;

/**
 * Clawd floating over other apps. Two windows: a large one that draws him, with room for his jumps and travel, which
 * lets touches through to the apps below; and a small invisible touch pad that the page keeps over his body.
 * Tap him for a move, double-tap to open the app and hide the bubble, long-press to open the app, drag to move him.
 */
public class BubbleService extends Service {
    static volatile boolean running = false;
    private static final String CHANNEL = "clawd";
    private static final String ACTION_STOP = "com.dekelelely.clawd.STOP";

    private final Handler main = new Handler(Looper.getMainLooper());
    private WindowManager windowManager;
    private WebView web;
    private View pad;
    private WindowManager.LayoutParams stageParams;
    private WindowManager.LayoutParams padParams;
    private float density;
    private boolean dragging;

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
        startInForeground();
        if (web == null) showBubble();
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
    private void showBubble() {
        running = true;
        windowManager = (WindowManager) getSystemService(WINDOW_SERVICE);
        DisplayMetrics metrics = getResources().getDisplayMetrics();
        density = metrics.density;

        // The stage: wide and short (the same shape as the page's pet camera), touches pass through it.
        int width = Math.round(214 * density);
        int height = Math.round(139 * density);
        stageParams = overlay(width, height,
                WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE
                        | WindowManager.LayoutParams.FLAG_NOT_TOUCHABLE
                        | WindowManager.LayoutParams.FLAG_HARDWARE_ACCELERATED);
        stageParams.x = metrics.widthPixels - width;
        stageParams.y = metrics.heightPixels - height - Math.round(110 * density);
        if (Build.VERSION.SDK_INT >= 31) {
            // Android 12+ only lets touches through an overlay from another app at or below this opacity.
            InputManager input = getSystemService(InputManager.class);
            stageParams.alpha = input.getMaximumObscuringOpacityForTouch();
        }

        web = new WebView(this);
        web.setBackgroundColor(Color.TRANSPARENT);
        web.setLayerType(View.LAYER_TYPE_HARDWARE, null);
        WebSettings settings = web.getSettings();
        settings.setJavaScriptEnabled(true);
        settings.setDomStorageEnabled(true);
        web.addJavascriptInterface(new Bridge(), "ClawdNative");
        web.loadUrl("file:///android_asset/www/index.html#mode=pet&platform=android");
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

    private void openApp() {
        Intent intent = new Intent(this, MainActivity.class);
        intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
        startActivity(intent);
    }

    /**
     * Taps, double-taps and long presses become actions; a drag moves both windows. Drags use raw screen
     * coordinates, so moving the pad under the finger doesn't feed back into the drag.
     */
    private class PadTouch implements View.OnTouchListener {
        private final int slop = ViewConfiguration.get(BubbleService.this).getScaledTouchSlop();
        private final GestureDetector gestures = new GestureDetector(BubbleService.this,
                new GestureDetector.SimpleOnGestureListener() {
                    @Override
                    public boolean onDown(MotionEvent e) {
                        return true;
                    }

                    @Override
                    public boolean onSingleTapUp(MotionEvent e) {
                        web.evaluateJavascript("window.clawdPet && window.clawdPet.tap()", null);
                        return true;
                    }

                    @Override
                    public boolean onDoubleTap(MotionEvent e) {
                        openApp();
                        stopSelf();
                        return true;
                    }

                    @Override
                    public void onLongPress(MotionEvent e) {
                        openApp();
                    }
                });
        private float downX, downY;
        private int stageX, stageY, padX, padY;

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
                    break;
                case MotionEvent.ACTION_MOVE: {
                    int dx = Math.round(event.getRawX() - downX);
                    int dy = Math.round(event.getRawY() - downY);
                    if (!dragging && Math.hypot(dx, dy) > slop * 1.5) {
                        dragging = true;
                        MotionEvent cancel = MotionEvent.obtain(event);
                        cancel.setAction(MotionEvent.ACTION_CANCEL);
                        gestures.onTouchEvent(cancel);
                        cancel.recycle();
                    }
                    if (dragging) {
                        stageParams.x = stageX + dx;
                        stageParams.y = stageY + dy;
                        padParams.x = padX + dx;
                        padParams.y = padY + dy;
                        windowManager.updateViewLayout(web, stageParams);
                        windowManager.updateViewLayout(pad, padParams);
                        return true;
                    }
                    break;
                }
                case MotionEvent.ACTION_UP:
                case MotionEvent.ACTION_CANCEL:
                    if (dragging) {
                        dragging = false;
                        return true;
                    }
                    break;
                default:
                    break;
            }
            return gestures.onTouchEvent(event);
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
        public void startBubble() {
            // Already floating.
        }

        @JavascriptInterface
        public boolean bubbleRunning() {
            return true;
        }

        /** Where Clawd's body is in the page, in CSS pixels; the touch pad moves over it. */
        @JavascriptInterface
        public void setHitRect(float x, float y, float w, float h) {
            main.post(() -> {
                if (pad == null || dragging) return;
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
