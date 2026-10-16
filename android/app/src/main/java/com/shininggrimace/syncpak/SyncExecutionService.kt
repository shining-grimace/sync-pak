package com.shininggrimace.syncpak

import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.IBinder

class SyncExecutionService : Service() {
    @Volatile private var cancelRequested = false
    @Volatile private var destroyed = false
    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        if (intent?.action == ACTION_CANCEL) {
            cancelRequested = true
            nativeSyncExecutionCancelled()
            if (SyncNative.state().optJSONObject("active") == null) stopExecution()
            SyncWidgets.refresh(this)
            return START_NOT_STICKY
        }

        val connectionName = intent?.getStringExtra(EXTRA_CONNECTION_NAME)
            ?.takeUnless { it.isBlank() } ?: "a sync operation"
        // Enter foreground before loading settings/history or initializing the worker.
        if (!publishRunning(connectionName)) return START_NOT_STICKY
        if (intent?.action == ACTION_RUN) cancelRequested = false
        Thread {
            val active = SyncNative.state().optJSONObject("active")
            if (intent?.action == ACTION_RUN && active == null) {
                val id = intent.getStringExtra(EXTRA_CONNECTION_ID).orEmpty()
                val error = runCatching { SyncNative.start(id) }.getOrNull()
                    ?: "Sync could not start. Open SyncPak and try again."
                if (cancelRequested) nativeSyncExecutionCancelled()
                if (error.isNotEmpty() && SyncNative.state().optJSONObject("active") == null) {
                    stopSelf(startId)
                    SyncExecutionNotification.complete(this, "Sync could not start", error)
                }
            } else if (active == null) {
                stopSelf(startId)
            } else {
                android.os.Handler(mainLooper).post {
                    if (!destroyed) publishRunning(active.optString("name"))
                }
            }
            SyncWidgets.refresh(this)
        }.start()
        return START_NOT_STICKY
    }

    private fun publishRunning(name: String): Boolean = try {
        SyncExecutionNotification.createChannel(this)
        startForeground(SyncExecutionNotification.ID, SyncExecutionNotification.build(this, name), ServiceInfo.FOREGROUND_SERVICE_TYPE_DATA_SYNC)
        true
    } catch (_: RuntimeException) {
        nativeSyncExecutionCancelled()
        stopSelf()
        runCatching { SyncExecutionNotification.complete(this, "Sync could not start", "Android could not start the sync service. Open SyncPak and try again.") }
        SyncWidgets.refresh(this)
        false
    }

    override fun onTimeout(startId: Int, foregroundServiceType: Int) {
        cancelRequested = true
        nativeSyncExecutionCancelled()
        stopExecution()
    }

    override fun onDestroy() {
        destroyed = true
        SyncExecutionNotification.clearRunning(this)
        SyncWidgets.refresh(this)
        super.onDestroy()
    }

    override fun onBind(intent: Intent?): IBinder? = null

    private fun stopExecution() {
        stopForeground(STOP_FOREGROUND_REMOVE)
        stopSelf()
    }

    companion object {
        internal const val ACTION_RUN = "com.shininggrimace.syncpak.action.RUN_SYNC"
        internal const val EXTRA_CONNECTION_ID = "connection_id"
        internal const val ACTION_CANCEL = "com.shininggrimace.syncpak.action.CANCEL_SYNC"
        private const val EXTRA_CONNECTION_NAME = "connection_name"

        @JvmStatic
        fun startIntent(context: Context, connectionName: String): Intent =
            Intent(context, SyncExecutionService::class.java)
                .putExtra(EXTRA_CONNECTION_NAME, connectionName)

        @JvmStatic
        private external fun nativeSyncExecutionCancelled()
    }
}
