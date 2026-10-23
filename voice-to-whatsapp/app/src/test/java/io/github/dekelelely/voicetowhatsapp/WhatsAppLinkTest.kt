package io.github.dekelelely.voicetowhatsapp

import org.junit.Assert.assertEquals
import org.junit.Test

class WhatsAppLinkTest {

    @Test
    fun spacesBecomePercent20() {
        assertEquals(
            "https://wa.me/972501234567?text=buy%20milk",
            WhatsApp.link("972501234567", "buy milk"),
        )
    }

    @Test
    fun hebrewIsUtf8PercentEncoded() {
        assertEquals(
            "https://wa.me/972501234567?text=%D7%A9%D7%9C%D7%95%D7%9D",
            WhatsApp.link("972501234567", "שלום"),
        )
    }

    @Test
    fun reservedCharactersAndNewlinesAreEncoded() {
        assertEquals(
            "https://wa.me/1?text=a%2Bb%26c%3Dd%23e%3F%0Af",
            WhatsApp.link("1", "a+b&c=d#e?\nf"),
        )
    }
}
