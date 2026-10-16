package com.shininggrimace.syncpak

import android.appwidget.AppWidgetManager
import android.appwidget.AppWidgetProvider
import android.content.Context
import android.content.Intent
import android.os.Bundle

open class SyncWidgetProvider : AppWidgetProvider() {
    override fun onUpdate(context: Context, manager: AppWidgetManager, ids: IntArray) { SyncWidgets.refresh(context) }
    override fun onAppWidgetOptionsChanged(context: Context, manager: AppWidgetManager, id: Int, options: Bundle) { SyncWidgets.refresh(context) }
    override fun onDeleted(context: Context, ids: IntArray) {
        val preferences = context.getSharedPreferences(SyncWidgets.PREFERENCES, Context.MODE_PRIVATE).edit()
        ids.forEach { preferences.remove(it.toString()) }
        preferences.apply()
    }
}
class CompactSyncWidgetProvider : SyncWidgetProvider()
