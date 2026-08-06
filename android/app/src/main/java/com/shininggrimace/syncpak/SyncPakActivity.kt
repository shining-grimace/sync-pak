package com.shininggrimace.syncpak

import android.Manifest
import android.app.NativeActivity
import android.content.Intent
import android.content.pm.PackageManager

class SyncPakActivity : NativeActivity() {
    fun pickFolder() {
        runOnUiThread {
            val intent = Intent(Intent.ACTION_OPEN_DOCUMENT_TREE).apply {
                addFlags(
                    Intent.FLAG_GRANT_READ_URI_PERMISSION or
                        Intent.FLAG_GRANT_WRITE_URI_PERMISSION or
                        Intent.FLAG_GRANT_PERSISTABLE_URI_PERMISSION or
                        Intent.FLAG_GRANT_PREFIX_URI_PERMISSION,
                )
            }
            startActivityForResult(intent, PICK_FOLDER_REQUEST)
        }
    }

    fun startSyncExecution(connectionName: String) {
        startForegroundService(SyncExecutionService.startIntent(this, connectionName))
    }

    fun stopSyncExecution() {
        stopService(Intent(this, SyncExecutionService::class.java))
    }

    fun hasInternetPermission(): Boolean =
        checkSelfPermission(Manifest.permission.INTERNET) == PackageManager.PERMISSION_GRANTED

    @Suppress("DEPRECATION")
    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        super.onActivityResult(requestCode, resultCode, data)
        if (requestCode != PICK_FOLDER_REQUEST) {
            return
        }

        val uri = data?.data
        if (resultCode != RESULT_OK || uri == null) {
            nativeFolderPickCancelled()
            return
        }

        val grantFlags = data.flags and
            (Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION)
        try {
            contentResolver.takePersistableUriPermission(uri, grantFlags)
            nativeFolderPicked(uri.toString())
        } catch (_: SecurityException) {
            nativeFolderPickFailed()
        }
    }

    companion object {
        private const val PICK_FOLDER_REQUEST = 4101

        @JvmStatic
        private external fun nativeFolderPicked(uri: String)

        @JvmStatic
        private external fun nativeFolderPickCancelled()

        @JvmStatic
        private external fun nativeFolderPickFailed()
    }
}
