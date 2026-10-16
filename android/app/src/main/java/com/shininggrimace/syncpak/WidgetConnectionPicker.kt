package com.shininggrimace.syncpak

import android.Manifest
import android.app.Activity
import android.appwidget.AppWidgetManager
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Bundle
import android.widget.*

/** Shared setup/reconfiguration UI; selecting a connection never starts a run. */
class WidgetConnectionPicker : Activity() {
    private var reload: (() -> Unit)? = null
    private var widgetId = AppWidgetManager.INVALID_APPWIDGET_ID
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setResult(RESULT_CANCELED)
        widgetId = intent.getIntExtra(AppWidgetManager.EXTRA_APPWIDGET_ID, AppWidgetManager.INVALID_APPWIDGET_ID)
        if (!SyncWidgets.validWidget(this, widgetId)) { finish(); return }
        if (android.os.Build.VERSION.SDK_INT >= 33 && checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED) {
            requestPermissions(arrayOf(Manifest.permission.POST_NOTIFICATIONS), 4104)
        }
        val padding = (20 * resources.displayMetrics.density).toInt()
        val panel = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL; setPadding(padding, padding, padding, padding) }
        panel.addView(TextView(this).apply { text = "Choose a SyncPak connection"; textSize = 22f })
        panel.addView(TextView(this).apply { text = "Only connections with Show Preview turned off are listed. Unverified connections must be verified in SyncPak before running."; setPadding(0, padding, 0, padding) })
        val emptyMessage = TextView(this).apply { visibility = android.view.View.GONE }
        panel.addView(emptyMessage)
        val list = ListView(this)
        panel.addView(list, LinearLayout.LayoutParams(-1, 0, 1f))
        panel.addView(Button(this).apply { text = "Open SyncPak"; setOnClickListener { startActivity(Intent(this@WidgetConnectionPicker, SyncPakActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_CLEAR_TOP or Intent.FLAG_ACTIVITY_SINGLE_TOP)) } })
        setContentView(panel)
        val bounds = windowManager.currentWindowMetrics.bounds
        val width = minOf(bounds.width() - padding * 2, (440 * resources.displayMetrics.density).toInt())
        val height = minOf((bounds.height() * 0.75).toInt(), (600 * resources.displayMetrics.density).toInt())
        window.setLayout(width, height)
        reload = { Thread {
            val state = SyncNative.state()
            val connections = state.optJSONArray("connections")
            val items = (0 until (connections?.length() ?: 0)).map { connections!!.getJSONObject(it) }
            runOnUiThread {
                if (isFinishing || isDestroyed) return@runOnUiThread
                emptyMessage.visibility = android.view.View.GONE
                list.visibility = android.view.View.VISIBLE
                window.setLayout(width, height)
                if (items.isEmpty()) {
                    emptyMessage.text = state.optString("error", "No connections available. Turn off Show Preview in a connection’s settings to use it here.")
                    emptyMessage.visibility = android.view.View.VISIBLE
                    list.visibility = android.view.View.GONE
                    window.setLayout(width, android.view.ViewGroup.LayoutParams.WRAP_CONTENT)
                }
                list.adapter = ArrayAdapter(this, android.R.layout.simple_list_item_single_choice, items.map { it.getString("name") + if (it.optBoolean("verified")) "" else " — Verify in app" })
                list.choiceMode = ListView.CHOICE_MODE_SINGLE
                val selected = getSharedPreferences(SyncWidgets.PREFERENCES, Context.MODE_PRIVATE).getString(widgetId.toString(), null)
                items.indexOfFirst { it.getString("id") == selected }.takeIf { it >= 0 }?.let { list.setItemChecked(it, true) }
                list.setOnItemClickListener { _, _, position, _ ->
                    getSharedPreferences(SyncWidgets.PREFERENCES, Context.MODE_PRIVATE).edit().putString(widgetId.toString(), items[position].getString("id")).apply()
                    SyncWidgets.refresh(this)
                    setResult(RESULT_OK, Intent().putExtra(AppWidgetManager.EXTRA_APPWIDGET_ID, widgetId))
                    finish()
                }
            }
        }.start() }
    }
    override fun onResume() {
        super.onResume()
        reload?.invoke()
    }
}
