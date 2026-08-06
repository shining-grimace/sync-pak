package com.shininggrimace.syncpak

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.graphics.drawable.Icon
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

        createNotificationChannel()
        startForeground(
            NOTIFICATION_ID,
            buildNotification(connectionName),
            ServiceInfo.FOREGROUND_SERVICE_TYPE_DATA_SYNC,
        )
        return START_NOT_STICKY
    }

    override fun onTimeout(startId: Int, foregroundServiceType: Int) {
        nativeSyncExecutionCancelled()
        stopExecution()
    }

    override fun onBind(intent: Intent?): IBinder? = null

    private fun buildNotification(connectionName: String): Notification {
        val openIntent = Intent(this, SyncPakActivity::class.java)
        val openPendingIntent = PendingIntent.getActivity(
            this,
            0,
            openIntent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )

        val cancelIntent = Intent(this, SyncExecutionService::class.java).setAction(ACTION_CANCEL)
        val cancelPendingIntent = PendingIntent.getService(
            this,
            1,
            cancelIntent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )
        val cancelAction = Notification.Action.Builder(
            Icon.createWithResource(this, android.R.drawable.ic_menu_close_clear_cancel),
            "Cancel",
            cancelPendingIntent,
        ).build()

        return Notification.Builder(this, CHANNEL_ID)
            .setSmallIcon(android.R.drawable.stat_sys_upload)
            .setContentTitle("Sync operation")
            .setContentText("SyncPak is running $connectionName")
            .setContentIntent(openPendingIntent)
            .setCategory(Notification.CATEGORY_PROGRESS)
            .setOngoing(true)
            .setOnlyAlertOnce(true)
            .addAction(cancelAction)
            .build()
    }

    private fun createNotificationChannel() {
        val channel = NotificationChannel(
            CHANNEL_ID,
            "Sync operations",
            NotificationManager.IMPORTANCE_LOW,
        ).apply {
            description = "Progress for active SyncPak operations"
        }
        getSystemService(NotificationManager::class.java).createNotificationChannel(channel)
    }

    private fun stopExecution() {
        stopForeground(STOP_FOREGROUND_REMOVE)
        stopSelf()
    }

    companion object {
        private const val ACTION_CANCEL = "com.shininggrimace.syncpak.action.CANCEL_SYNC"
        private const val EXTRA_CONNECTION_NAME = "connection_name"
        private const val CHANNEL_ID = "sync_operations"
        private const val NOTIFICATION_ID = 4102

        @JvmStatic
        fun startIntent(context: Context, connectionName: String): Intent =
            Intent(context, SyncExecutionService::class.java)
                .putExtra(EXTRA_CONNECTION_NAME, connectionName)

        @JvmStatic
        private external fun nativeSyncExecutionCancelled()
    }
}
