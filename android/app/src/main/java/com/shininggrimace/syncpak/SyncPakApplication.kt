package com.shininggrimace.syncpak

import android.app.Application
import android.os.FileObserver
import android.os.Handler
import android.os.Looper
import java.io.File

class SyncPakApplication : Application() {
    private var observer: FileObserver? = null
    override fun onCreate() {
        super.onCreate()
        SyncNative.initialize(SyncPlatform(this), filesDir.absolutePath)
        val directory = File(filesDir, "sync-pak").apply { mkdirs() }
        val handler = Handler(Looper.getMainLooper())
        val refresh = Runnable { SyncWidgets.refresh(this) }
        observer = object : FileObserver(directory, MOVED_TO or CLOSE_WRITE or DELETE) {
            override fun onEvent(event: Int, path: String?) {
                if (path == "config.json" || path == "activity.json") {
                    handler.removeCallbacks(refresh)
                    handler.postDelayed(refresh, 150)
                }
            }
        }.also { it.startWatching() }
    }
}
