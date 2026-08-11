package com.shininggrimace.syncpak

import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.IBinder

class SyncExecutionService : Service() {
    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        if (intent?.action == ACTION_CANCEL) {
            nativeSyncExecutionCancelled()
            stopExecution()
            return START_NOT_STICKY
        }

        val connectionName = intent
            ?.getStringExtra(EXTRA_CONNECTION_NAME)
            ?.takeUnless { it.isBlank() }
            ?: "a sync operation"

        SyncExecutionNotification.createChannel(this)
        startForeground(
            SyncExecutionNotification.ID,
            SyncExecutionNotification.build(this, connectionName),
            ServiceInfo.FOREGROUND_SERVICE_TYPE_DATA_SYNC,
        )
        return START_NOT_STICKY
    }

    override fun onTimeout(startId: Int, foregroundServiceType: Int) {
        nativeSyncExecutionCancelled()
        stopExecution()
    }

    override fun onBind(intent: Intent?): IBinder? = null

    private fun stopExecution() {
        stopForeground(STOP_FOREGROUND_REMOVE)
        stopSelf()
    }

    companion object {
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
