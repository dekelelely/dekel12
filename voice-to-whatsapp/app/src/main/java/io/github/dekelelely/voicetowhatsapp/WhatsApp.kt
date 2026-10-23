package io.github.dekelelely.voicetowhatsapp

import android.content.ActivityNotFoundException
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import java.net.URLEncoder

object WhatsApp {
    // Personal WhatsApp first, then WhatsApp Business. Both must also be listed under <queries> in
    // the manifest, or Android 11+ reports them as not installed.
    private val PACKAGES = listOf("com.whatsapp", "com.whatsapp.w4b")

    /**
     * https://wa.me/<number>?text=<text>. With the user's own number, WhatsApp opens their chat with
     * themselves and puts the text in the message box. It is not sent until the user taps Send.
     */
    fun link(number: String, text: String): String {
        // URLEncoder does HTML-form encoding, which turns a space into "+". In a URL query "+" isn't
        // reliably read back as a space, so switch to %20. A "+" typed in the text is already "%2B".
        val encoded = URLEncoder.encode(text, "UTF-8").replace("+", "%20")
        return "https://wa.me/$number?text=$encoded"
    }

    /** The WhatsApp package to open, or null if neither variant is installed and enabled. */
    fun installedPackage(pm: PackageManager): String? =
        PACKAGES.firstOrNull { pm.getLaunchIntentForPackage(it) != null }

    /**
     * Opens the pre-filled chat. Returns false when WhatsApp isn't installed, without opening anything,
     * so the caller can say so. Otherwise a bare wa.me link would just land on a web page.
     */
    fun open(context: Context, number: String, text: String): Boolean {
        val pkg = installedPackage(context.packageManager) ?: return false
        // NEW_TASK: WhatsApp opens in its own task, as if launched normally, not stacked inside ours.
        val view = Intent(Intent.ACTION_VIEW, Uri.parse(link(number, text)))
            .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        return try {
            context.startActivity(Intent(view).setPackage(pkg))
            true
        } catch (e: ActivityNotFoundException) {
            // WhatsApp is installed but doesn't handle wa.me links on this phone (e.g. link handling
            // turned off). Let the system choose; the wa.me page then hands over to WhatsApp.
            try {
                context.startActivity(view)
                true
            } catch (e: ActivityNotFoundException) {
                false
            }
        }
    }
}
