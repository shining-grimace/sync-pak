package com.shininggrimace.syncpak

import android.app.PendingIntent
import android.appwidget.AppWidgetManager
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.view.View
import android.widget.RemoteViews
import org.json.JSONObject
import java.util.concurrent.Executors

/** Complete RemoteViews updates keep multiple widget instances in agreement. */
object SyncWidgets {
    const val PREFERENCES = "sync_widgets"
    private const val EXPANDED_MIN_HEIGHT_DP = 112
    private val executor = Executors.newSingleThreadExecutor()
    private val flags = PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
    fun refresh(context: Context) {
        val application = context.applicationContext
        executor.execute {
            val state = SyncNative.state()
            WidgetAppearance.remember(application, state)
            runCatching { renderAll(application, state) }
                .onFailure { android.util.Log.w("SyncPakWidgets", "Widget refresh failed", it) }
        }
    }
    fun providers() = listOf(SyncWidgetProvider::class.java)
    fun validWidget(context: Context, id: Int): Boolean {
        val provider = AppWidgetManager.getInstance(context).getAppWidgetInfo(id)?.provider ?: return false
        return providers().any { provider == ComponentName(context, it) }
    }
    private fun renderAll(context: Context, state: JSONObject) {
        val manager = AppWidgetManager.getInstance(context)
        for (provider in providers()) {
            manager.getAppWidgetIds(ComponentName(context, provider)).forEach { id ->
                val height = manager.getAppWidgetOptions(id)
                    .getInt(AppWidgetManager.OPTION_APPWIDGET_MIN_HEIGHT)
                val compact = height in 1 until EXPANDED_MIN_HEIGHT_DP
                manager.updateAppWidget(id, render(context, state, id, compact))
            }
        }
    }
    private fun render(context: Context, state: JSONObject, id: Int, compact: Boolean): RemoteViews {
        val layout = when (WidgetAppearance.preference(context)) {
            "light" -> if (compact) R.layout.widget_sync_compact_light else R.layout.widget_sync_light
            "dark" -> if (compact) R.layout.widget_sync_compact_dark else R.layout.widget_sync_dark
            else -> if (compact) R.layout.widget_sync_compact else R.layout.widget_sync
        }
        val views = RemoteViews(context.packageName, layout)
        val selected = context.getSharedPreferences(PREFERENCES, Context.MODE_PRIVATE).getString(id.toString(), null)
        val connections = state.optJSONArray("connections")
        val connection = (0 until (connections?.length() ?: 0))
            .map { connections!!.getJSONObject(it) }
            .find { it.getString("id") == selected }
        val active = state.optJSONObject("active")
        val busy = active != null
        val runningHere = active != null && active.optString("id") == selected
        val runningElsewhere = busy && !runningHere
        val empty = connections == null || connections.length() == 0
        val verified = connection?.optBoolean("verified") == true
        val name = connection?.getString("name")
            ?: if (state.has("error")) "Connections unavailable" else if (empty) "No connections" else "Choose connection"
        val running = "Running: ${active?.optString("name")}"
        val status = when {
            runningElsewhere -> "Another connection is running"
            runningHere -> running
            else -> ""
        }
        val showCompactStatus = compact && busy
        views.setTextViewText(R.id.widget_connection, if (showCompactStatus) status else name)
        views.setTextViewCompoundDrawablesRelative(
            R.id.widget_connection,
            0,
            0,
            if (showCompactStatus) 0 else R.drawable.ic_disclosure_triangle,
            0
        )
        views.setContentDescription(
            R.id.widget_connection_picker,
            if (showCompactStatus) status else "Choose connection. $name"
        )
        val action = when {
            empty -> "Open SyncPak"
            connection != null && !verified -> "Verify in app"
            else -> "Run"
        }
        views.setTextViewText(
            R.id.widget_run,
            if (compact) action.substringBefore(' ') else action
        )
        views.setContentDescription(R.id.widget_run, action)
        views.setViewVisibility(R.id.widget_run, if (busy) View.GONE else View.VISIBLE)
        views.setBoolean(
            R.id.widget_run,
            "setEnabled",
            !busy && (empty || connection != null)
        )
        views.setBoolean(R.id.widget_connection, "setEnabled", !busy)
        views.setBoolean(R.id.widget_connection_picker, "setEnabled", !busy)
        views.setInt(
            R.id.widget_icon,
            "setImageAlpha",
            if (runningElsewhere) 115 else 255
        )
        if (!compact) {
            views.setTextViewText(R.id.widget_status, status)
            views.setViewVisibility(R.id.widget_status, if (busy) View.VISIBLE else View.GONE)
            views.setViewVisibility(R.id.widget_disclaimer, if (busy) View.GONE else View.VISIBLE)
            views.setBoolean(R.id.widget_app_name, "setEnabled", !runningElsewhere)
            views.setBoolean(R.id.widget_disclaimer, "setEnabled", !runningElsewhere)
            views.setTextViewText(
                R.id.widget_disclaimer,
                if (state.has("error")) "Connections unavailable. Open SyncPak to check settings."
                else if (empty) "No connections available. Turn off Show Preview in a connection’s settings to use it here."
                else ""
            )
        }
        val picker = Intent(context, WidgetConnectionPicker::class.java).putExtra(AppWidgetManager.EXTRA_APPWIDGET_ID, id)
            .setData(Uri.parse("syncpak://widget/$id/pick"))
        views.setOnClickPendingIntent(R.id.widget_connection_picker, PendingIntent.getActivity(context, id, picker, flags))
        val open = PendingIntent.getActivity(context, id, Intent(context, SyncPakActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_CLEAR_TOP or Intent.FLAG_ACTIVITY_SINGLE_TOP).setData(Uri.parse("syncpak://widget/$id/open")), flags)
        views.setOnClickPendingIntent(R.id.widget_icon, open)
        if (empty || !verified) {
            views.setOnClickPendingIntent(R.id.widget_run, open)
        } else {
            val run = Intent(context, SyncExecutionService::class.java).setAction(SyncExecutionService.ACTION_RUN)
                .putExtra(SyncExecutionService.EXTRA_CONNECTION_ID, selected).putExtra("connection_name", name)
                .setData(Uri.parse("syncpak://widget/$id/run/$selected"))
            views.setOnClickPendingIntent(R.id.widget_run, PendingIntent.getForegroundService(context, id, run, flags))
        }
        return views
    }
}
