package com.shininggrimace.syncpak

import android.Manifest
import android.app.Activity
import android.appwidget.AppWidgetManager
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Bundle
import android.view.View
import android.view.ViewGroup
import android.widget.*

/** Shared setup/reconfiguration UI; selecting a connection never starts a run. */
class WidgetConnectionPicker : Activity() {
    private var returnToPicker: (() -> Unit)? = null
    private var reload: (() -> Unit)? = null
    private var appearance = "system"
    private var widgetId = AppWidgetManager.INVALID_APPWIDGET_ID
    override fun onCreate(savedInstanceState: Bundle?) {
        appearance = WidgetAppearance.preference(this)
        setTheme(WidgetAppearance.pickerTheme(appearance))
        super.onCreate(savedInstanceState)
        setResult(RESULT_CANCELED)
        widgetId = intent.getIntExtra(AppWidgetManager.EXTRA_APPWIDGET_ID, AppWidgetManager.INVALID_APPWIDGET_ID)
        if (!SyncWidgets.validWidget(this, widgetId)) { finish(); return }
        if (android.os.Build.VERSION.SDK_INT >= 33 && checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED) {
            requestPermissions(arrayOf(Manifest.permission.POST_NOTIFICATIONS), 4104)
        }
        val padding = (20 * resources.displayMetrics.density).toInt()
        val panel = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL; setPadding(padding, padding, padding, padding) }
        panel.addView(TextView(this).apply { text = "Choose a connection for this widget. Preview-enabled syncs can run here when no files need to be overwritten or deleted. Verify unverified connections in SyncPak before running."; setPadding(0, padding, 0, padding) })
        val emptyMessage = TextView(this).apply { visibility = View.GONE }
        panel.addView(emptyMessage)
        val list = ListView(this)
        panel.addView(list, LinearLayout.LayoutParams(-1, 0, 1f))
        panel.addView(Button(this).apply { text = "Open SyncPak"; setOnClickListener { startActivity(Intent(this@WidgetConnectionPicker, SyncPakActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_CLEAR_TOP or Intent.FLAG_ACTIVITY_SINGLE_TOP)) } })
        val content = FrameLayout(this)
        content.addView(panel)
        setContentView(content)
        if (android.os.Build.VERSION.SDK_INT >= 33) {
            onBackInvokedDispatcher.registerOnBackInvokedCallback(
                android.window.OnBackInvokedDispatcher.PRIORITY_DEFAULT,
            ) {
                returnToPicker?.invoke() ?: finish()
            }
        }
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
                WidgetAppearance.remember(this, state)
                if (WidgetAppearance.preference(this) != appearance) {
                    recreate()
                    return@runOnUiThread
                }
                emptyMessage.visibility = View.GONE
                list.visibility = View.VISIBLE
                if (returnToPicker == null) window.setLayout(width, height)
                if (items.isEmpty()) {
                    emptyMessage.text = state.optString("error", "No connections available for widgets.")
                    emptyMessage.visibility = View.VISIBLE
                    list.visibility = View.GONE
                    if (returnToPicker == null) window.setLayout(width, ViewGroup.LayoutParams.WRAP_CONTENT)
                }
                list.adapter = ArrayAdapter(this, android.R.layout.simple_list_item_single_choice, items.map { it.getString("name") + if (it.optBoolean("verified")) "" else " — Verify in app" })
                list.choiceMode = ListView.CHOICE_MODE_SINGLE
                val selected = getSharedPreferences(SyncWidgets.PREFERENCES, Context.MODE_PRIVATE).getString(widgetId.toString(), null)
                items.indexOfFirst { it.getString("id") == selected }.takeIf { it >= 0 }?.let { list.setItemChecked(it, true) }
                list.setOnItemClickListener { _, _, position, _ ->
                    val item = items[position]
                    val id = item.getString("id")
                    if (item.optBoolean("requires_opt_in")) {
                        val consent = consentPanel(item.getString("name"), id, padding)
                        panel.visibility = View.GONE
                        content.addView(consent)
                        val windowTitle = window.decorView.findViewById<View>(android.R.id.title)
                        val titleVisibility = windowTitle?.visibility
                        windowTitle?.visibility = View.GONE
                        window.setLayout(width, ViewGroup.LayoutParams.WRAP_CONTENT)
                        returnToPicker = {
                            content.removeView(consent)
                            panel.visibility = View.VISIBLE
                            windowTitle?.let { it.visibility = titleVisibility ?: View.VISIBLE }
                            window.setLayout(width, height)
                            returnToPicker = null
                            list.clearChoices()
                            items.indexOfFirst { it.getString("id") == selected }
                                .takeIf { it >= 0 }?.let { list.setItemChecked(it, true) }
                            list.invalidateViews()
                        }
                    } else {
                        select(id, createOnly = false)
                    }
                }
            }
        }.start() }
    }
    private fun consentPanel(name: String, id: String, padding: Int): ScrollView {
        val body = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(padding, padding, padding, padding)
            addView(TextView(this@WidgetConnectionPicker).apply {
                text = name
                setTextAppearance(android.R.style.TextAppearance_Material_Title)
                setAccessibilityHeading(true)
            })
            addView(TextView(this@WidgetConnectionPicker).apply {
                text = "This will proceed only if no files will be overwritten or deleted."
                setPadding(0, padding, 0, padding / 2)
            })
            addView(TextView(this@WidgetConnectionPicker).apply {
                text = "The widget checks the local and remote folders before syncing. Uploads require an endpoint that supports create-only writes. If review is needed, open the result from Activity or tap the notification."
                setPadding(0, 0, 0, padding)
            })
            addView(LinearLayout(this@WidgetConnectionPicker).apply {
                orientation = LinearLayout.HORIZONTAL
                addView(Button(this@WidgetConnectionPicker, null, 0, R.style.SyncPakWidgetPickerOutlineButton).apply {
                    text = "Back"
                    setOnClickListener { returnToPicker?.invoke() }
                }, LinearLayout.LayoutParams(0, ViewGroup.LayoutParams.WRAP_CONTENT, 1f).apply {
                    marginEnd = padding / 2
                })
                addView(Button(this@WidgetConnectionPicker).apply {
                    text = "Use connection"
                    setOnClickListener { select(id, createOnly = true) }
                }, LinearLayout.LayoutParams(0, ViewGroup.LayoutParams.WRAP_CONTENT, 1f))
            })
        }
        return ScrollView(this).apply { addView(body) }
    }

    @Suppress("DEPRECATION", "OVERRIDE_DEPRECATION")
    override fun onBackPressed() {
        returnToPicker?.invoke() ?: super.onBackPressed()
    }

    private fun select(id: String, createOnly: Boolean) {
        getSharedPreferences(SyncWidgets.PREFERENCES, Context.MODE_PRIVATE).edit()
            .putString(widgetId.toString(), id)
            .putBoolean(SyncWidgets.createOnlyKey(widgetId), createOnly)
            .apply()
        SyncWidgets.refresh(this)
        setResult(RESULT_OK, Intent().putExtra(AppWidgetManager.EXTRA_APPWIDGET_ID, widgetId))
        finish()
    }

    override fun onResume() {
        super.onResume()
        reload?.invoke()
    }
}
