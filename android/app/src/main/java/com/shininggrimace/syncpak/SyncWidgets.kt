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
    private val executor = Executors.newSingleThreadExecutor()
    private val flags = PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
    fun refresh(context: Context) {
        val application = context.applicationContext
        executor.execute {
            val state = SyncNative.state()
            runCatching { renderAll(application, state) }
                .onFailure { android.util.Log.w("SyncPakWidgets", "Widget refresh failed", it) }
        }
    }
    fun providers() = listOf(SyncWidgetProvider::class.java, CompactSyncWidgetProvider::class.java)
    fun validWidget(context: Context, id: Int): Boolean {
        val provider = AppWidgetManager.getInstance(context).getAppWidgetInfo(id)?.provider ?: return false
        return providers().any { provider == ComponentName(context, it) }
    }
    private fun renderAll(context: Context, state: JSONObject) {
        val manager = AppWidgetManager.getInstance(context)
        for (provider in providers()) {
            val compact = provider == CompactSyncWidgetProvider::class.java
            manager.getAppWidgetIds(ComponentName(context, provider)).forEach { id ->
                manager.updateAppWidget(id, render(context, state, id, compact))
            }
        }
    }
    private fun render(context: Context, state: JSONObject, id: Int, compact: Boolean): RemoteViews {
        val views = RemoteViews(context.packageName, if (compact) R.layout.widget_compact else R.layout.widget_sync)
        val selected = context.getSharedPreferences(PREFERENCES, Context.MODE_PRIVATE).getString(id.toString(), null)
        val connections = state.optJSONArray("connections")
        val connection = (0 until (connections?.length() ?: 0)).map { connections!!.getJSONObject(it) }.find { it.getString("id") == selected }
        val active = state.optJSONObject("active")
        val busy = active != null
        val empty = connections == null || connections.length() == 0
        val verified = connection?.optBoolean("verified") == true
        val name = connection?.getString("name") ?: if (state.has("error")) "Connections unavailable" else if (empty) "No connections" else "Choose connection"
        val running = "Running: ${active?.optString("name")}"
        views.setTextViewText(R.id.widget_connection, if (compact && busy) running else "$name ▾")
        views.setContentDescription(R.id.widget_connection, if (compact && busy) running else "Choose connection. $name")
        views.setTextViewText(R.id.widget_run, when { busy -> "Running…"; empty -> "Open SyncPak"; connection != null && !verified -> "Verify in app"; else -> "Run" })
        views.setBoolean(R.id.widget_run, "setEnabled", !busy && (empty || connection != null))
        views.setBoolean(R.id.widget_connection, "setEnabled", !busy)
        if (!compact) {
            views.setTextViewText(R.id.widget_status, running)
            views.setViewVisibility(R.id.widget_status, if (busy) View.VISIBLE else View.GONE)
            views.setTextViewText(R.id.widget_disclaimer, if (state.has("error")) "Connections unavailable. Open SyncPak to check settings." else if (empty) "No connections available. Turn off Show Preview in a connection’s settings to use it here." else "Only connections with preview turned off are available.")
        }
        val picker = Intent(context, WidgetConnectionPicker::class.java).putExtra(AppWidgetManager.EXTRA_APPWIDGET_ID, id)
            .setData(Uri.parse("syncpak://widget/$id/pick"))
        views.setOnClickPendingIntent(R.id.widget_connection, PendingIntent.getActivity(context, id, picker, flags))
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
