package io.github.dekelelely.voicetowhatsapp

import android.app.Activity
import android.content.Context
import android.content.Intent
import android.os.Bundle
import android.view.View
import android.widget.Button
import android.widget.EditText
import android.widget.RadioGroup
import android.widget.TextView
import android.widget.Toast

/** Shows the dictated text for editing, then hands it to WhatsApp. */
class ReviewActivity : Activity() {

    private lateinit var prefs: Prefs
    private lateinit var textField: EditText
    private lateinit var recipient: TextView

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_review)
        requireView<View>(R.id.root).padForSystemBarsAndKeyboard()
        prefs = Prefs(this)

        textField = requireView(R.id.text)
        recipient = requireView(R.id.recipient)
        // After a rotation the EditText restores its own (possibly edited) text.
        if (savedInstanceState == null) setDictatedText(intent.getStringExtra(EXTRA_TEXT).orEmpty())

        val languages = requireView<RadioGroup>(R.id.language)
        languages.check(if (prefs.language == Prefs.HEBREW) R.id.lang_hebrew else R.id.lang_english)
        applyTextDirection()
        // Saved immediately: the choice applies to "Dictate again" and to the next widget tap.
        languages.setOnCheckedChangeListener { _, checkedId ->
            prefs.language = if (checkedId == R.id.lang_hebrew) Prefs.HEBREW else Prefs.ENGLISH
            applyTextDirection()
        }

        requireView<Button>(R.id.change_number).setOnClickListener {
            startActivity(SetupActivity.intent(this, SetupActivity.Then.GO_BACK))
        }
        requireView<Button>(R.id.dictate_again).setOnClickListener {
            if (!Speech.start(this, prefs.language)) toast(R.string.error_recognizer_unavailable)
        }
        requireView<Button>(R.id.send).setOnClickListener { send() }
    }

    override fun onResume() {
        super.onResume()
        // Refreshed here because the number may have just been changed via "Change".
        recipient.text = prefs.phoneNumber?.let { getString(R.string.send_to, PhoneNumber.display(it)) }
    }

    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        super.onActivityResult(requestCode, resultCode, data)
        if (requestCode != Speech.REQUEST_CODE) return
        when (val result = Speech.parseResult(resultCode, data)) {
            is Speech.Result.Text -> setDictatedText(result.text) // replaces the previous take
            Speech.Result.Cancelled -> Unit // keep the current text
            is Speech.Result.Failed -> toast(result.messageRes) // the button is right there to retry
        }
    }

    private fun setDictatedText(text: String) {
        textField.setText(text)
        textField.setSelection(text.length)
    }

    /**
     * Hebrew and English can be mixed in the field. Each paragraph takes its direction from its own
     * first letter: a Hebrew line is right-aligned and an English line left-aligned (gravity="start"
     * follows the paragraph). The dictation language only sets the fallback for lines with no letters
     * yet. That also puts the cursor of an empty field on the right side.
     */
    private fun applyTextDirection() {
        textField.textDirection =
            if (prefs.language == Prefs.HEBREW) View.TEXT_DIRECTION_FIRST_STRONG_RTL
            else View.TEXT_DIRECTION_FIRST_STRONG_LTR
    }

    private fun send() {
        val number = prefs.phoneNumber
        if (number == null) {
            startActivity(SetupActivity.intent(this, SetupActivity.Then.GO_BACK))
            return
        }
        val text = textField.text.toString().trim()
        if (text.isEmpty()) {
            textField.error = getString(R.string.error_empty_text)
            return
        }
        // Once WhatsApp has the text there's nothing left to do here. On failure, stay so the text isn't lost.
        if (WhatsApp.open(this, number, text)) finish() else toast(R.string.error_whatsapp_missing)
    }

    private fun toast(messageRes: Int) = Toast.makeText(this, messageRes, Toast.LENGTH_LONG).show()

    companion object {
        private const val EXTRA_TEXT = "text"

        fun intent(context: Context, text: String): Intent =
            Intent(context, ReviewActivity::class.java).putExtra(EXTRA_TEXT, text)
    }
}
