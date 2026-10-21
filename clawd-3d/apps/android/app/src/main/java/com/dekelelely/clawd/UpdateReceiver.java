package com.dekelelely.clawd;

import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;

/** Receives Android's answers about an update install (see Updater) and passes them to the app's screen. */
public class UpdateReceiver extends BroadcastReceiver {
    @Override
    public void onReceive(Context context, Intent intent) {
        MainActivity.updateStatus(intent);
    }
}
