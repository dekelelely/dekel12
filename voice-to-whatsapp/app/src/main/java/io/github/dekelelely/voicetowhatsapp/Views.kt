package io.github.dekelelely.voicetowhatsapp

import android.app.Activity
import android.os.Build
import android.view.View
import android.view.WindowInsets

/**
 * findViewById for views that are always in the layout, typed as non-null. Fails loudly if the id is
 * missing. (The platform's requireViewById does the same but needs API 28; minSdk is 26.)
 */
fun <T : View> Activity.requireView(id: Int): T =
    findViewById<T>(id) ?: throw IllegalStateException("No view with id ${resources.getResourceName(id)}")

/**
 * Android 15+ draws apps that target SDK 35 edge-to-edge: the window extends under the status bar,
 * the navigation bar and the keyboard, and adjustResize no longer shrinks it. This pads the view by
 * those insets, on top of its own XML padding, so nothing ends up hidden behind them.
 *
 * Older versions lay the window out between the bars and resize it for the keyboard
 * (windowSoftInputMode="adjustResize"), so there is nothing to do there.
 */
fun View.padForSystemBarsAndKeyboard() {
    if (Build.VERSION.SDK_INT < Build.VERSION_CODES.VANILLA_ICE_CREAM) return
    val left = paddingLeft
    val top = paddingTop
    val right = paddingRight
    val bottom = paddingBottom
    setOnApplyWindowInsetsListener { view, insets ->
        val bars = insets.getInsets(
            WindowInsets.Type.systemBars() or WindowInsets.Type.displayCutout() or WindowInsets.Type.ime()
        )
        view.setPadding(left + bars.left, top + bars.top, right + bars.right, bottom + bars.bottom)
        insets
    }
}
