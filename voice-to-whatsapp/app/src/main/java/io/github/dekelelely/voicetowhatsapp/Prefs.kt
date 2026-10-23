package io.github.dekelelely.voicetowhatsapp

import android.content.Context
import java.util.Locale

/** The app's only persistent state: the user's own number and the last dictation language. */
class Prefs(context: Context) {
    private val prefs = context.getSharedPreferences("settings", Context.MODE_PRIVATE)

    /** Digits only, international format (e.g. "972501234567"), or null until first-run setup is done. */
    var phoneNumber: String?
        get() = prefs.getString(KEY_PHONE, null)?.takeIf { it.isNotEmpty() }
        set(value) = prefs.edit().putString(KEY_PHONE, value).apply()

    /** Language tag handed to the recognizer: [HEBREW] or [ENGLISH]. */
    var language: String
        get() = prefs.getString(KEY_LANGUAGE, null) ?: defaultLanguage()
        set(value) = prefs.edit().putString(KEY_LANGUAGE, value).apply()

    companion object {
        const val HEBREW = "he-IL"
        const val ENGLISH = "en-US"

        private const val KEY_PHONE = "phone_number"
        private const val KEY_LANGUAGE = "language"

        // Until the user picks a language, follow the phone's. Java reports Hebrew as "iw" on older
        // Android versions and "he" on newer ones.
        private fun defaultLanguage(): String =
            if (Locale.getDefault().language in setOf("he", "iw")) HEBREW else ENGLISH
    }
}
