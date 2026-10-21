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
import android.os.Build;
import android.os.Handler;
import android.os.IBinder;
import android.os.Looper;
import android.util.DisplayMetrics;
import android.view.Gravity;
import android.view.MotionEvent;
import android.view.View;
import android.view.ViewConfiguration;
import android.view.WindowManager;
import android.webkit.JavascriptInterface;
import android.webkit.WebSettings;
import android.webkit.WebView;

/**
 * Clawd in a small transparent window over other apps, like a chat bubble.
 * Tap him for a move, drag to move him, long-press to open the app. The notification hides him.
 */
public class BubbleService extends Service {
    static volatile boolean running = false;
    private static final String CHANNEL = "clawd";
    private static final String ACTION_STOP = "com.dekelelely.clawd.STOP";

    private WindowManager windowManager;
    private WebView web;
    private WindowManager.LayoutParams params;

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

    @SuppressLint({"SetJavaScriptEnabled", "ClickableViewAccessibility"})
    private void showBubble() {
        running = true;
        windowManager = (WindowManager) getSystemService(WINDOW_SERVICE);
        DisplayMetrics metrics = getResources().getDisplayMetrics();
        float dp = metrics.density;
        int width = Math.round(150 * dp);
        int height = Math.round(138 * dp);

        params = new WindowManager.LayoutParams(
                width, height,
                WindowManager.LayoutParams.TYPE_APPLICATION_OVERLAY,
                WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE | WindowManager.LayoutParams.FLAG_HARDWARE_ACCELERATED,
                PixelFormat.TRANSLUCENT);
        params.gravity = Gravity.TOP | Gravity.START;
        params.x = metrics.widthPixels - width - Math.round(8 * dp);
        params.y = metrics.heightPixels - height - Math.round(120 * dp);

        web = new WebView(this);
        web.setBackgroundColor(Color.TRANSPARENT);
        web.setLayerType(View.LAYER_TYPE_HARDWARE, null);
        WebSettings settings = web.getSettings();
        settings.setJavaScriptEnabled(true);
        settings.setDomStorageEnabled(true);
        web.addJavascriptInterface(new Bridge(), "ClawdNative");
        web.setOnTouchListener(new DragToMove());
        web.loadUrl("file:///android_asset/www/index.html#mode=pet&platform=android");
        windowManager.addView(web, params);
    }

    /**
     * Drags move the bubble using raw screen coordinates, so moving the window under the finger
     * doesn't feed back into the drag. Taps and long presses go through to the page.
     */
    private class DragToMove implements View.OnTouchListener {
        private final int slop = ViewConfiguration.get(BubbleService.this).getScaledTouchSlop();
        private float downX, downY;
        private int startX, startY;
        private boolean dragging;

        @Override
        public boolean onTouch(View view, MotionEvent event) {
            switch (event.getActionMasked()) {
                case MotionEvent.ACTION_DOWN:
                    downX = event.getRawX();
                    downY = event.getRawY();
                    startX = params.x;
                    startY = params.y;
                    dragging = false;
                    return false;
                case MotionEvent.ACTION_MOVE: {
                    float dx = event.getRawX() - downX;
                    float dy = event.getRawY() - downY;
                    if (!dragging && Math.hypot(dx, dy) > slop * 1.5) {
                        dragging = true;
                        // Tell the page the touch is over, so it doesn't count this as a tap.
                        MotionEvent cancel = MotionEvent.obtain(event);
                        cancel.setAction(MotionEvent.ACTION_CANCEL);
                        view.onTouchEvent(cancel);
                        cancel.recycle();
                    }
                    if (dragging) {
                        params.x = startX + Math.round(dx);
                        params.y = startY + Math.round(dy);
                        windowManager.updateViewLayout(web, params);
                        return true;
                    }
                    return false;
                }
                case MotionEvent.ACTION_UP:
                case MotionEvent.ACTION_CANCEL:
                    if (dragging) {
                        dragging = false;
                        return true;
                    }
                    return false;
                default:
                    return false;
            }
        }
    }

    /** Calls the bubble's page can make (window.ClawdNative). They arrive on a background thread. */
    private class Bridge {
        private final Handler main = new Handler(Looper.getMainLooper());

        @JavascriptInterface
        public void openApp() {
            Intent intent = new Intent(BubbleService.this, MainActivity.class);
            intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
            startActivity(intent);
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
    }

    @Override
    public void onDestroy() {
        running = false;
        if (web != null) {
            windowManager.removeView(web);
            web.destroy();
            web = null;
        }
        super.onDestroy();
    }
}
