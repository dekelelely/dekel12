package io.github.dekelelely.voicetowhatsapp

import android.app.Activity
import android.app.AlertDialog
import android.content.Intent
import android.os.Bundle

/**
 * What a widget tap opens. It has a transparent theme and no layout of its own: it starts the
 * recognizer straight away, so the user sees the recognizer over their home screen. Then it hands the
 * text to ReviewActivity. Its only UI is an error dialog when something goes wrong.
 */
class VoiceActivity : Activity() {

    private lateinit var prefs: Prefs

    /** True while the recognizer is on screen and its result is pending. */
    private var awaitingResult = false

    /** The error dialog currently showing, if any. */
    private var dialog: AlertDialog? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        prefs = Prefs(this)

        if (savedInstanceState != null) {
            awaitingResult = savedInstanceState.getBoolean(KEY_AWAITING_RESULT)
            // Recreated (e.g. after Android killed the process) while an error dialog was showing. The
            // dialog is gone, and an invisible activity would just sit over the home screen eating taps.
            if (!awaitingResult) finish()
            // Otherwise the recognizer is still open and its result arrives in onActivityResult.
            return
        }

        if (prefs.phoneNumber == null) {
            startActivity(SetupActivity.intent(this, SetupActivity.Then.DICTATE))
            finish()
            return
        }
        listen()
    }

    override fun onSaveInstanceState(outState: Bundle) {
        super.onSaveInstanceState(outState)
        outState.putBoolean(KEY_AWAITING_RESULT, awaitingResult)
    }

    override fun onDestroy() {
        // Clear the field first, so the dismiss listener below doesn't treat this as the user closing it.
        dialog?.let { dialog = null; it.dismiss() }
        super.onDestroy()
    }

    private fun listen() {
        awaitingResult = Speech.start(this, prefs.language)
        if (!awaitingResult) showError(R.string.error_recognizer_unavailable, offerRetry = false)
    }

    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        super.onActivityResult(requestCode, resultCode, data)
        if (requestCode != Speech.REQUEST_CODE) return
        awaitingResult = false
        when (val result = Speech.parseResult(resultCode, data)) {
            is Speech.Result.Text -> {
                startActivity(ReviewActivity.intent(this, result.text))
                finish()
            }
            Speech.Result.Cancelled -> finish()
            is Speech.Result.Failed -> showError(result.messageRes, offerRetry = true)
        }
    }

    /** Closing the dialog in any way (button, back, tap outside) closes the activity, except "Try again". */
    private fun showError(messageRes: Int, offerRetry: Boolean) {
        val builder = AlertDialog.Builder(this)
            .setMessage(messageRes)
            .setNegativeButton(R.string.close, null)
        // The button's click runs before the dialog's dismiss, so listen() has already set
        // awaitingResult by the time the dismiss listener checks it.
        if (offerRetry) builder.setPositiveButton(R.string.try_again) { _, _ -> listen() }

        val shown = builder.create()
        shown.setOnDismissListener {
            // Ignore the dismissal of a dialog that has since been replaced by a newer one
            // (a retry that failed straight away), or that onDestroy is cleaning up.
            if (dialog !== shown) return@setOnDismissListener
            dialog = null
            if (!awaitingResult) finish()
        }
        dialog = shown
        shown.show()
    }

    private companion object {
        const val KEY_AWAITING_RESULT = "awaiting_result"
    }
}
