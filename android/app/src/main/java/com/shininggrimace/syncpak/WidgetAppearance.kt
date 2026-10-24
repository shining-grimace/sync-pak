package com.shininggrimace.syncpak

import android.content.Context
import org.json.JSONObject

/** Cache the native appearance preference so the picker is themed before inflation. */
object WidgetAppearance {
    private const val PREFERENCES = "widget_appearance"

    fun preference(context: Context): String =
        context.getSharedPreferences(PREFERENCES, Context.MODE_PRIVATE)
            .getString("appearance", "system") ?: "system"

    fun remember(context: Context, state: JSONObject) {
        if (!state.has("appearance")) return
        val appearance = state.optString("appearance", "system")
        if (appearance != preference(context)) {
            context.getSharedPreferences(PREFERENCES, Context.MODE_PRIVATE)
                .edit().putString("appearance", appearance).apply()
        }
    }

    fun pickerTheme(appearance: String): Int = when (appearance) {
        "light" -> R.style.SyncPakWidgetPicker_Light
        "dark" -> R.style.SyncPakWidgetPicker_Dark
        else -> R.style.SyncPakWidgetPicker
    }
}
