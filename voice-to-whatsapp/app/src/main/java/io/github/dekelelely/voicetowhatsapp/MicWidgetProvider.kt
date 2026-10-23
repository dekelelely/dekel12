package io.github.dekelelely.voicetowhatsapp

import android.app.PendingIntent
import android.appwidget.AppWidgetManager
import android.appwidget.AppWidgetProvider
import android.content.Context
import android.content.Intent
import android.widget.RemoteViews

/**
 * The home-screen mic. It has a single job: launch VoiceActivity when tapped.
 *
 * It always launches VoiceActivity, even before a number is saved. VoiceActivity checks for the
 * number when the widget is tapped and opens setup if it's missing. A PendingIntent is fixed when the
 * widget is drawn, so checking at tap time means the widget never needs redrawing when the number
 * changes.
 */
class MicWidgetProvider : AppWidgetProvider() {

    override fun onUpdate(context: Context, appWidgetManager: AppWidgetManager, appWidgetIds: IntArray) {
        val views = RemoteViews(context.packageName, R.layout.widget_mic)
        views.setOnClickPendingIntent(R.id.widget_root, clickIntent(context))
        appWidgetManager.updateAppWidget(appWidgetIds, views)
    }

    private fun clickIntent(context: Context): PendingIntent {
        // NEW_TASK is required to start an activity from outside one (the launcher). CLEAR_TASK makes
        // every tap start a fresh dictation instead of bringing back an old review screen.
        val intent = Intent(context, VoiceActivity::class.java)
            .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TASK)
        // FLAG_IMMUTABLE is mandatory from Android 12, and nothing needs to modify this intent.
        return PendingIntent.getActivity(
            context, 0, intent, PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT
        )
    }
}
