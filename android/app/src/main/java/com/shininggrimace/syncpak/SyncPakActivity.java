package com.shininggrimace.syncpak;

import android.app.NativeActivity;
import android.content.Intent;
import android.net.Uri;

public final class SyncPakActivity extends NativeActivity {
    private static final int PICK_FOLDER_REQUEST = 4101;

    public void pickFolder() {
        runOnUiThread(() -> {
            Intent intent = new Intent(Intent.ACTION_OPEN_DOCUMENT_TREE);
            intent.addFlags(
                    Intent.FLAG_GRANT_READ_URI_PERMISSION
                            | Intent.FLAG_GRANT_WRITE_URI_PERMISSION
                            | Intent.FLAG_GRANT_PERSISTABLE_URI_PERMISSION
                            | Intent.FLAG_GRANT_PREFIX_URI_PERMISSION);
            startActivityForResult(intent, PICK_FOLDER_REQUEST);
        });
    }

    public void startSyncExecution(String connectionName) {
        Intent intent = SyncExecutionService.startIntent(this, connectionName);
        startForegroundService(intent);
    }

    public void stopSyncExecution() {
        stopService(new Intent(this, SyncExecutionService.class));
    }

    @Override
    protected void onActivityResult(int requestCode, int resultCode, Intent data) {
        super.onActivityResult(requestCode, resultCode, data);
        if (requestCode != PICK_FOLDER_REQUEST) {
            return;
        }
        if (resultCode != RESULT_OK || data == null || data.getData() == null) {
            nativeFolderPickCancelled();
            return;
        }

        Uri uri = data.getData();
        int grantFlags = data.getFlags()
                & (Intent.FLAG_GRANT_READ_URI_PERMISSION
                | Intent.FLAG_GRANT_WRITE_URI_PERMISSION);
        try {
            getContentResolver().takePersistableUriPermission(uri, grantFlags);
            nativeFolderPicked(uri.toString());
        } catch (SecurityException error) {
            nativeFolderPickFailed();
        }
    }

    private static native void nativeFolderPicked(String uri);

    private static native void nativeFolderPickCancelled();

    private static native void nativeFolderPickFailed();
}
