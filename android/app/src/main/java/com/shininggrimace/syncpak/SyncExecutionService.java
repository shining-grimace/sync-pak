package com.shininggrimace.syncpak;

import android.app.Notification;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.app.PendingIntent;
import android.app.Service;
import android.content.Context;
import android.content.Intent;
import android.content.pm.ServiceInfo;
import android.graphics.drawable.Icon;
import android.os.IBinder;

public final class SyncExecutionService extends Service {
    private static final String ACTION_CANCEL =
            "com.shininggrimace.syncpak.action.CANCEL_SYNC";
    private static final String EXTRA_CONNECTION_NAME = "connection_name";
    private static final String CHANNEL_ID = "sync_operations";
    private static final int NOTIFICATION_ID = 4102;

    public static Intent startIntent(Context context, String connectionName) {
        return new Intent(context, SyncExecutionService.class)
                .putExtra(EXTRA_CONNECTION_NAME, connectionName);
    }

    @Override
    public int onStartCommand(Intent intent, int flags, int startId) {
        if (intent != null && ACTION_CANCEL.equals(intent.getAction())) {
            stopExecution();
            return START_NOT_STICKY;
        }

        String connectionName = intent == null
                ? "a sync operation"
                : intent.getStringExtra(EXTRA_CONNECTION_NAME);
        if (connectionName == null || connectionName.isBlank()) {
            connectionName = "a sync operation";
        }

        createNotificationChannel();
        startForeground(
                NOTIFICATION_ID,
                buildNotification(connectionName),
                ServiceInfo.FOREGROUND_SERVICE_TYPE_DATA_SYNC);
        return START_NOT_STICKY;
    }

    @Override
    public void onTimeout(int startId, int foregroundServiceType) {
        stopExecution();
    }

    @Override
    public IBinder onBind(Intent intent) {
        return null;
    }

    private Notification buildNotification(String connectionName) {
        Intent openIntent = new Intent(this, SyncPakActivity.class);
        PendingIntent openPendingIntent = PendingIntent.getActivity(
                this,
                0,
                openIntent,
                PendingIntent.FLAG_UPDATE_CURRENT | PendingIntent.FLAG_IMMUTABLE);

        Intent cancelIntent = new Intent(this, SyncExecutionService.class)
                .setAction(ACTION_CANCEL);
        PendingIntent cancelPendingIntent = PendingIntent.getService(
                this,
                1,
                cancelIntent,
                PendingIntent.FLAG_UPDATE_CURRENT | PendingIntent.FLAG_IMMUTABLE);
        Notification.Action cancelAction = new Notification.Action.Builder(
                Icon.createWithResource(this, android.R.drawable.ic_menu_close_clear_cancel),
                "Cancel",
                cancelPendingIntent)
                .build();

        return new Notification.Builder(this, CHANNEL_ID)
                .setSmallIcon(android.R.drawable.stat_sys_upload)
                .setContentTitle("Sync operation")
                .setContentText("SyncPak is running " + connectionName)
                .setContentIntent(openPendingIntent)
                .setCategory(Notification.CATEGORY_PROGRESS)
                .setOngoing(true)
                .setOnlyAlertOnce(true)
                .addAction(cancelAction)
                .build();
    }

    private void createNotificationChannel() {
        NotificationChannel channel = new NotificationChannel(
                CHANNEL_ID,
                "Sync operations",
                NotificationManager.IMPORTANCE_LOW);
        channel.setDescription("Progress for active SyncPak operations");
        getSystemService(NotificationManager.class).createNotificationChannel(channel);
    }

    private void stopExecution() {
        stopForeground(STOP_FOREGROUND_REMOVE);
        stopSelf();
    }
}
