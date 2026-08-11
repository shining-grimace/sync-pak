package com.shininggrimace.syncpak

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.graphics.drawable.Icon

object SyncExecutionNotification {
    const val ID = 4102
    private const val CHANNEL_ID = "sync_operations"

    fun createChannel(context: Context) {
        val channel = NotificationChannel(
            CHANNEL_ID,
            "Sync operations",
            NotificationManager.IMPORTANCE_LOW,
        ).apply {
            description = "Progress for active SyncPak operations"
        }
        context.getSystemService(NotificationManager::class.java).createNotificationChannel(channel)
    }

    fun build(
        context: Context,
        connectionName: String,
        currentFile: Int? = null,
        totalFiles: Int? = null,
    ): Notification {
        val openPendingIntent = PendingIntent.getActivity(
            context,
            0,
            Intent(context, SyncPakActivity::class.java),
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )
        val cancelPendingIntent = PendingIntent.getService(
            context,
            1,
            Intent(context, SyncExecutionService::class.java)
                .setAction(SyncExecutionService.ACTION_CANCEL),
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )
        val cancelAction = Notification.Action.Builder(
            Icon.createWithResource(context, android.R.drawable.ic_menu_close_clear_cancel),
            "Cancel",
            cancelPendingIntent,
        ).build()
        val builder = Notification.Builder(context, CHANNEL_ID)
            .setSmallIcon(android.R.drawable.stat_sys_upload)
            .setContentTitle("Sync operation")
            .setContentText(notificationText(connectionName, currentFile, totalFiles))
            .setContentIntent(openPendingIntent)
            .setCategory(Notification.CATEGORY_PROGRESS)
            .setOngoing(true)
            .setOnlyAlertOnce(true)
            .addAction(cancelAction)
        if (currentFile != null && totalFiles != null) {
            builder.setProgress(totalFiles, currentFile - 1, false)
        }
        return builder.build()
    }

    fun update(context: Context, connectionName: String, currentFile: Int, totalFiles: Int) {
        if (currentFile !in 1..totalFiles) {
            return
        }
        context.getSystemService(NotificationManager::class.java).notify(
            ID,
            build(context, connectionName, currentFile, totalFiles),
        )
    }

    private fun notificationText(
        connectionName: String,
        currentFile: Int?,
        totalFiles: Int?,
    ): String {
        val running = "SyncPak is running $connectionName"
        return if (currentFile == null || totalFiles == null) {
            running
        } else {
            "$running · File $currentFile of $totalFiles"
        }
    }
}
