package com.shininggrimace.syncpak

import android.content.Intent

object SyncResultNavigation {
    @Volatile var pending: String? = null
        private set
    fun accept(intent: Intent?) {
        intent?.getStringExtra("operation_id")?.let { pending = it }
    }
    @Synchronized fun clear(id: String) { if (pending == id) pending = null }
}
