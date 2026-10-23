package io.github.dekelelely.voicetowhatsapp

import io.github.dekelelely.voicetowhatsapp.PhoneNumber.Warning.STARTS_WITH_ZERO
import io.github.dekelelely.voicetowhatsapp.PhoneNumber.Warning.TOO_SHORT
import org.junit.Assert.assertEquals
import org.junit.Test

class PhoneNumberTest {

    @Test
    fun normalizeKeepsOnlyDigits() {
        assertEquals("972501234567", PhoneNumber.normalize("+972 (50) 123-4567"))
    }

    @Test
    fun normalizeConvertsNonAsciiDigits() {
        assertEquals("972", PhoneNumber.normalize("٩٧٢"))
    }

    @Test
    fun normalizeOfNoDigitsIsEmpty() {
        assertEquals("", PhoneNumber.normalize("abc +-()"))
    }

    @Test
    fun validInternationalNumberHasNoWarnings() {
        assertEquals(emptyList<PhoneNumber.Warning>(), PhoneNumber.warnings("972501234567"))
    }

    @Test
    fun leadingZeroWarns() {
        assertEquals(listOf(STARTS_WITH_ZERO), PhoneNumber.warnings("0501234567"))
    }

    @Test
    fun doubleZeroInternationalPrefixWarns() {
        assertEquals(listOf(STARTS_WITH_ZERO), PhoneNumber.warnings("00972501234567"))
    }

    @Test
    fun sevenDigitsIsTooShort() {
        assertEquals(listOf(TOO_SHORT), PhoneNumber.warnings("9725012"))
    }

    @Test
    fun eightDigitsIsLongEnough() {
        assertEquals(emptyList<PhoneNumber.Warning>(), PhoneNumber.warnings("97250123"))
    }

    @Test
    fun shortNumberWithLeadingZeroGetsBothWarnings() {
        assertEquals(listOf(STARTS_WITH_ZERO, TOO_SHORT), PhoneNumber.warnings("050"))
    }
}
