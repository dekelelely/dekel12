package io.github.dekelelely.voicetowhatsapp

import android.app.Activity
import android.content.ActivityNotFoundException
import android.content.Intent
import android.speech.RecognizerIntent

/**
 * Launching the system speech recognizer and reading its result. Shared by VoiceActivity (widget tap)
 * and ReviewActivity ("Dictate again").
 *
 * This uses the RecognizerIntent *activity* rather than SpeechRecognizer: the recognizer app (usually
 * the Google app) shows its own listening UI and records under its own microphone permission, so this
 * app needs no permissions at all.
 */
object Speech {
    const val REQUEST_CODE = 1

    sealed interface Result {
        data class Text(val text: String) : Result

        /** The user backed out of the recognizer. Not an error, so nothing is shown. */
        data object Cancelled : Result

        /** Something went wrong; worth offering "Try again". [messageRes] is a string resource. */
        data class Failed(val messageRes: Int) : Result
    }

    /** Starts listening in [language] ("he-IL" or "en-US"). Returns false if no recognizer is available. */
    fun start(activity: Activity, language: String): Boolean {
        val prompt = if (language == Prefs.HEBREW) R.string.prompt_hebrew else R.string.prompt_english
        val intent = Intent(RecognizerIntent.ACTION_RECOGNIZE_SPEECH)
            .putExtra(RecognizerIntent.EXTRA_LANGUAGE_MODEL, RecognizerIntent.LANGUAGE_MODEL_FREE_FORM)
            .putExtra(RecognizerIntent.EXTRA_LANGUAGE, language)
            .putExtra(RecognizerIntent.EXTRA_PROMPT, activity.getString(prompt))
            .putExtra(RecognizerIntent.EXTRA_MAX_RESULTS, 1)
        // EXTRA_PREFER_OFFLINE is deliberately left unset: the recognizer decides between on-device
        // and online recognition itself.

        // resolveActivity only sees recognizers thanks to the <queries> entry in the manifest.
        if (intent.resolveActivity(activity.packageManager) == null) return false
        return try {
            activity.startActivityForResult(intent, REQUEST_CODE)
            true
        } catch (e: ActivityNotFoundException) {
            false // Resolved but couldn't start, e.g. the recognizer app was just disabled.
        }
    }

    fun parseResult(resultCode: Int, data: Intent?): Result = when (resultCode) {
        Activity.RESULT_OK -> {
            val text = data?.getStringArrayListExtra(RecognizerIntent.EXTRA_RESULTS)?.firstOrNull()?.trim()
            if (text.isNullOrEmpty()) Result.Failed(R.string.error_no_speech) else Result.Text(text)
        }
        // Google's recognizer handles silence with its own "try again" UI and returns RESULT_CANCELED if
        // the user gives up. Other recognizers report no speech as RESULT_NO_MATCH or an empty list.
        Activity.RESULT_CANCELED -> Result.Cancelled
        RecognizerIntent.RESULT_NO_MATCH -> Result.Failed(R.string.error_no_speech)
        RecognizerIntent.RESULT_NETWORK_ERROR,
        RecognizerIntent.RESULT_SERVER_ERROR -> Result.Failed(R.string.error_network)
        RecognizerIntent.RESULT_AUDIO_ERROR -> Result.Failed(R.string.error_audio)
        else -> Result.Failed(R.string.error_speech_generic) // RESULT_CLIENT_ERROR or non-standard codes
    }
}
