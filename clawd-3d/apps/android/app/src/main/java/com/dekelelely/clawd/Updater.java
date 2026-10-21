package com.dekelelely.clawd;

import android.app.PendingIntent;
import android.content.Context;
import android.content.Intent;
import android.content.IntentSender;
import android.content.pm.PackageInfo;
import android.content.pm.PackageInstaller;
import android.content.pm.PackageManager;
import android.os.Build;
import android.os.Handler;
import android.os.Looper;

import org.json.JSONArray;
import org.json.JSONException;
import org.json.JSONObject;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.net.HttpURLConnection;
import java.net.URL;
import java.nio.charset.StandardCharsets;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

/**
 * Updates from GitHub Releases. The "Clawd apps" workflow publishes every build as release v1.N.0 with Clawd.apk,
 * where N is that build's versionCode. check() reads the latest release; install() streams its APK into Android's
 * package installer, which asks the person to confirm. Every state change reaches the listener on the main thread.
 */
final class Updater {
    interface Listener {
        void onUpdateState(String json);
    }

    private static final String LATEST = "https://api.github.com/repos/dekelelely/dekel12/releases/latest";
    private static final Pattern TAG = Pattern.compile("^v?1\\.(\\d+)");

    private final Context context;
    private final Listener listener;
    private final Handler main = new Handler(Looper.getMainLooper());
    private final ExecutorService worker = Executors.newSingleThreadExecutor();

    // idle, checking, none, available, downloading, installing or error
    private String state = "idle";
    private String latestVersion = "";
    private String apkUrl = "";
    private long apkSize = -1;
    private int percent = 0;
    private String message = "";

    Updater(Context context, Listener listener) {
        this.context = context.getApplicationContext();
        this.listener = listener;
    }

    /** The state for the page's update button, as JSON: state, current, version, percent, message. */
    synchronized String stateJson() {
        JSONObject o = new JSONObject();
        try {
            o.put("state", state).put("current", currentVersion()).put("version", latestVersion)
                    .put("percent", percent).put("message", message);
        } catch (JSONException ignored) {
            // Plain strings and numbers always fit.
        }
        return o.toString();
    }

    private void set(String next, String why) {
        synchronized (this) {
            state = next;
            message = why;
        }
        String json = stateJson();
        main.post(() -> listener.onUpdateState(json));
    }

    private synchronized boolean busy() {
        return state.equals("checking") || state.equals("downloading");
    }

    /** Looks for a newer release. */
    void check() {
        if (busy()) return;
        set("checking", "");
        worker.execute(() -> {
            try {
                JSONObject release = new JSONObject(readText(LATEST));
                Matcher tag = TAG.matcher(release.optString("tag_name"));
                String url = "";
                long size = -1;
                JSONArray assets = release.optJSONArray("assets");
                for (int i = 0; assets != null && i < assets.length(); i++) {
                    JSONObject asset = assets.getJSONObject(i);
                    if ("Clawd.apk".equals(asset.optString("name"))) {
                        url = asset.optString("browser_download_url");
                        size = asset.optLong("size", -1);
                    }
                }
                if (!tag.find() || url.isEmpty()) throw new IOException("the latest release has no Clawd.apk");
                int build = Integer.parseInt(tag.group(1));
                synchronized (this) {
                    latestVersion = "1." + build;
                    apkUrl = url;
                    apkSize = size;
                }
                set(build > currentBuild() ? "available" : "none", "");
            } catch (Exception e) {
                set("error", "Couldn't check for updates: " + e.getMessage());
            }
        });
    }

    /** Downloads the new APK into an install session and asks Android to install it. */
    void install() {
        final String url;
        final long size;
        synchronized (this) {
            if (!state.equals("available")) return;
            url = apkUrl;
            size = apkSize;
            percent = 0;
        }
        set("downloading", "");
        worker.execute(() -> {
            PackageInstaller installer = context.getPackageManager().getPackageInstaller();
            int sessionId = -1;
            try {
                PackageInstaller.SessionParams params =
                        new PackageInstaller.SessionParams(PackageInstaller.SessionParams.MODE_FULL_INSTALL);
                params.setAppPackageName(context.getPackageName());
                if (size > 0) params.setSize(size);
                sessionId = installer.createSession(params);
                HttpURLConnection conn = open(url, "application/octet-stream");
                long total = size > 0 ? size : conn.getContentLengthLong();
                try (PackageInstaller.Session session = installer.openSession(sessionId)) {
                    try (InputStream in = conn.getInputStream();
                         OutputStream out = session.openWrite("Clawd.apk", 0, total > 0 ? total : -1)) {
                        byte[] buffer = new byte[64 * 1024];
                        long done = 0;
                        int n, shown = 0;
                        while ((n = in.read(buffer)) > 0) {
                            out.write(buffer, 0, n);
                            done += n;
                            int now = total > 0 ? (int) (done * 100 / total) : 0;
                            if (now != shown) {
                                shown = now;
                                synchronized (this) {
                                    percent = now;
                                }
                                set("downloading", "");
                            }
                        }
                        session.fsync(out);
                    } finally {
                        conn.disconnect();
                    }
                    session.commit(statusReceiver());
                }
                set("installing", "");
            } catch (Exception e) {
                if (sessionId != -1) {
                    try {
                        installer.abandonSession(sessionId);
                    } catch (Exception ignored) {
                        // Already gone.
                    }
                }
                set("error", "Couldn't download the update: " + e.getMessage());
            }
        });
    }

    /** Android's answer about the install (see UpdateReceiver). Returns the confirmation screen to show, if any. */
    Intent onStatus(Intent intent) {
        int status = intent.getIntExtra(PackageInstaller.EXTRA_STATUS, PackageInstaller.STATUS_FAILURE);
        if (status == PackageInstaller.STATUS_PENDING_USER_ACTION) return intent.getParcelableExtra(Intent.EXTRA_INTENT);
        if (status == PackageInstaller.STATUS_SUCCESS) set("none", "");
        else if (status == PackageInstaller.STATUS_FAILURE_ABORTED) set("available", "");   // declined: offer it again
        else set("error", "Android couldn't install the update: " + intent.getStringExtra(PackageInstaller.EXTRA_STATUS_MESSAGE));
        return null;
    }

    private IntentSender statusReceiver() {
        Intent intent = new Intent(context, UpdateReceiver.class);
        int flags = PendingIntent.FLAG_UPDATE_CURRENT | (Build.VERSION.SDK_INT >= 31 ? PendingIntent.FLAG_MUTABLE : 0);
        return PendingIntent.getBroadcast(context, 0, intent, flags).getIntentSender();
    }

    private int currentBuild() {
        try {
            PackageInfo info = context.getPackageManager().getPackageInfo(context.getPackageName(), 0);
            return Build.VERSION.SDK_INT >= 28 ? (int) info.getLongVersionCode() : info.versionCode;
        } catch (PackageManager.NameNotFoundException e) {
            return 0;
        }
    }

    private String currentVersion() {
        try {
            return context.getPackageManager().getPackageInfo(context.getPackageName(), 0).versionName;
        } catch (PackageManager.NameNotFoundException e) {
            return "";
        }
    }

    private static HttpURLConnection open(String url, String accept) throws IOException {
        HttpURLConnection conn = (HttpURLConnection) new URL(url).openConnection();
        conn.setConnectTimeout(15000);
        conn.setReadTimeout(30000);
        conn.setRequestProperty("User-Agent", "Clawd-Android");
        conn.setRequestProperty("Accept", accept);
        int code = conn.getResponseCode();
        if (code != HttpURLConnection.HTTP_OK) {
            conn.disconnect();
            throw new IOException("HTTP " + code);
        }
        return conn;
    }

    private static String readText(String url) throws IOException {
        HttpURLConnection conn = open(url, "application/vnd.github+json");
        try (InputStream in = conn.getInputStream()) {
            ByteArrayOutputStream bytes = new ByteArrayOutputStream();
            byte[] buffer = new byte[16 * 1024];
            int n;
            while ((n = in.read(buffer)) > 0) bytes.write(buffer, 0, n);
            return bytes.toString(StandardCharsets.UTF_8.name());
        } finally {
            conn.disconnect();
        }
    }
}
