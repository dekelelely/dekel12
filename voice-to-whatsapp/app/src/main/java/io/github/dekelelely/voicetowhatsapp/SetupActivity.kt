package io.github.dekelelely.voicetowhatsapp

import android.app.Activity
import android.app.AlertDialog
import android.appwidget.AppWidgetManager
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.os.Bundle
import android.text.Editable
import android.text.TextWatcher
import android.view.View
import android.view.inputmethod.EditorInfo
import android.view.inputmethod.InputMethodManager
import android.widget.Button
import android.widget.EditText
import android.widget.TextView
import android.widget.Toast

/**
 * The first-run setup screen and the Settings screen. They're the same form: "Welcome" with only the
 * number field until a number is saved, then "Settings", which also shows how to add the widget.
 * Opened from the launcher icon, from the widget when no number is saved yet, and from "Change" on
 * the review screen.
 */
class SetupActivity : Activity() {

    private lateinit var prefs: Prefs
    private lateinit var phoneField: EditText
    private lateinit var preview: TextView
    private lateinit var warning: TextView

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_setup)
        requireView<View>(R.id.root).padForSystemBarsAndKeyboard()
        prefs = Prefs(this)

        phoneField = requireView(R.id.phone)
        preview = requireView(R.id.phone_preview)
        warning = requireView(R.id.phone_warning)

        if (savedInstanceState == null) phoneField.setText(prefs.phoneNumber?.let { "+$it" })
        phoneField.addTextChangedListener(object : TextWatcher {
            override fun beforeTextChanged(s: CharSequence?, start: Int, count: Int, after: Int) = Unit
            override fun onTextChanged(s: CharSequence?, start: Int, before: Int, count: Int) = Unit
            override fun afterTextChanged(s: Editable?) = updateFeedback()
        })
        phoneField.setOnEditorActionListener { _, actionId, _ ->
            if (actionId == EditorInfo.IME_ACTION_DONE) {
                save()
                true
            } else {
                false
            }
        }

        requireView<Button>(R.id.save).setOnClickListener { save() }
        requireView<TextView>(R.id.usage_body).text =
            getString(R.string.usage_body, getString(R.string.widget_label))
        requireView<Button>(R.id.add_widget).setOnClickListener { requestPinWidget() }
        requireView<Button>(R.id.try_now).setOnClickListener {
            startActivity(Intent(this, VoiceActivity::class.java))
        }

        updateFeedback()
        updateMode()
    }

    /** Live feedback while typing: the digits that will be stored, plus any warnings. */
    private fun updateFeedback() {
        val digits = PhoneNumber.normalize(phoneField.text.toString())
        preview.text = getString(R.string.phone_preview, digits)
        preview.visibility = if (digits.isEmpty()) View.GONE else View.VISIBLE
        val messages = warningMessages(digits)
        warning.text = messages.joinToString("\n")
        warning.visibility = if (digits.isEmpty() || messages.isEmpty()) View.GONE else View.VISIBLE
    }

    private fun warningMessages(digits: String): List<String> = PhoneNumber.warnings(digits).map {
        when (it) {
            PhoneNumber.Warning.STARTS_WITH_ZERO -> getString(R.string.warning_starts_with_zero)
            PhoneNumber.Warning.TOO_SHORT -> getString(R.string.warning_too_short)
        }
    }

    private fun save() {
        val digits = PhoneNumber.normalize(phoneField.text.toString())
        if (digits.isEmpty()) {
            phoneField.error = getString(R.string.error_phone_empty)
            return
        }
        val messages = warningMessages(digits)
        if (messages.isEmpty()) {
            commit(digits)
            return
        }
        // A warning, not a block: the user may know better (e.g. an unusually short number).
        AlertDialog.Builder(this)
            .setTitle(R.string.confirm_number_title)
            .setMessage(PhoneNumber.display(digits) + "\n\n" + messages.joinToString("\n\n"))
            .setPositiveButton(R.string.save_anyway) { _, _ -> commit(digits) }
            .setNegativeButton(R.string.edit, null)
            .show()
    }

    private fun commit(digits: String) {
        prefs.phoneNumber = digits
        val then = intent.getStringExtra(EXTRA_THEN)?.let { Then.valueOf(it) } ?: Then.STAY
        if (then == Then.DICTATE) {
            // The user tapped the widget before setting up. Now that there's a number, carry on with
            // what they were trying to do.
            startActivity(Intent(this, VoiceActivity::class.java))
            finish()
            return
        }
        Toast.makeText(this, R.string.saved, Toast.LENGTH_SHORT).show()
        if (then == Then.GO_BACK) {
            finish()
            return
        }
        phoneField.setText("+$digits")
        phoneField.clearFocus()
        getSystemService(InputMethodManager::class.java)
            ?.hideSoftInputFromWindow(phoneField.windowToken, 0)
        updateMode()
    }

    /** "Welcome" before a number is saved, "Settings" (plus the widget instructions) after. */
    private fun updateMode() {
        val hasNumber = prefs.phoneNumber != null
        requireView<TextView>(R.id.title).setText(
            if (hasNumber) R.string.setup_title_settings else R.string.setup_title_first_run
        )
        requireView<View>(R.id.usage_section).visibility = if (hasNumber) View.VISIBLE else View.GONE
    }

    /** Asks the launcher to place the widget directly (Android 8+), where the launcher supports it. */
    private fun requestPinWidget() {
        val manager = AppWidgetManager.getInstance(this)
        if (manager.isRequestPinAppWidgetSupported) {
            manager.requestPinAppWidget(ComponentName(this, MicWidgetProvider::class.java), null, null)
        } else {
            Toast.makeText(this, R.string.add_widget_unsupported, Toast.LENGTH_LONG).show()
        }
    }

    /** What happens after Save. Opened from the launcher icon (no extra), it's STAY. */
    enum class Then {
        /** Stay on the screen, now in Settings mode. */
        STAY,

        /** Go on to dictation: the widget was tapped before any number was saved. */
        DICTATE,

        /** Return to the screen that opened this one ("Change" on the review screen). */
        GO_BACK,
    }

    companion object {
        private const val EXTRA_THEN = "then"

        fun intent(context: Context, then: Then): Intent =
            Intent(context, SetupActivity::class.java).putExtra(EXTRA_THEN, then.name)
    }
}
