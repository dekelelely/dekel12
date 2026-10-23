package io.github.dekelelely.voicetowhatsapp

import android.text.BidiFormatter
import android.text.TextDirectionHeuristics

/** Cleaning up and sanity-checking the user's own number. normalize() and warnings() are unit-tested. */
object PhoneNumber {
    enum class Warning { STARTS_WITH_ZERO, TOO_SHORT }

    private const val MIN_DIGITS = 8

    /** Keeps only the digits: "+972 (50) 123-4567" becomes "972501234567". */
    fun normalize(raw: String): String = buildString {
        for (c in raw) {
            // Character.digit also maps non-ASCII digits (e.g. Arabic-Indic) to 0-9, so a number typed
            // on such a keyboard isn't silently dropped. wa.me needs plain ASCII digits.
            val digit = Character.digit(c, 10)
            if (digit >= 0) append('0' + digit)
        }
    }

    /**
     * Warnings, not errors: the user may still save. A leading 0 is a domestic trunk prefix (or the
     * "00" international prefix), which wa.me can't use; fewer than 8 digits is too short for a
     * country code plus a subscriber number almost everywhere.
     */
    fun warnings(digits: String): List<Warning> = buildList {
        if (digits.startsWith('0')) add(Warning.STARTS_WITH_ZERO)
        if (digits.length < MIN_DIGITS) add(Warning.TOO_SHORT)
    }

    /**
     * "+972501234567", wrapped so it stays left-to-right inside Hebrew text. Without this, a
     * number at the start or end of an RTL line can show its "+" on the wrong side.
     */
    fun display(digits: String): String =
        BidiFormatter.getInstance().unicodeWrap("+$digits", TextDirectionHeuristics.LTR)
}
