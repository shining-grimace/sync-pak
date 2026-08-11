package com.shininggrimace.syncpak

import android.Manifest
import android.app.NativeActivity
import android.content.Intent
import android.content.pm.PackageManager

class SyncPakActivity : NativeActivity() {
    private val documentTrees by lazy { DocumentTreeAccess(this) }

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

    fun updateSyncExecution(connectionName: String, currentFile: Int, totalFiles: Int) {
        SyncExecutionNotification.update(this, connectionName, currentFile, totalFiles)
    }

    fun stopSyncExecution() {
        stopService(Intent(this, SyncExecutionService::class.java))
    }

    fun hasInternetPermission(): Boolean =
        checkSelfPermission(Manifest.permission.INTERNET) == PackageManager.PERMISSION_GRANTED

    fun verifyFolder(uri: String): Int = documentTrees.verify(uri)

    fun inventoryFolder(uri: String): String? = documentTrees.inventory(uri)

    fun folderEntryMetadata(uri: String, path: String): String? =
        documentTrees.metadata(uri, path)

    fun openFolderFileForRead(uri: String, path: String): Int =
        documentTrees.openForRead(uri, path)

    fun openFolderFileForWrite(uri: String, path: String): Int =
        documentTrees.openForWrite(uri, path)

    fun createFolderPath(uri: String, path: String): Int = documentTrees.createDirectories(uri, path)

    fun deleteFolderEntry(uri: String, path: String): Int = documentTrees.delete(uri, path)

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

        init {
            System.loadLibrary("sync_pak")
        }

        @JvmStatic
        private external fun nativeFolderPicked(uri: String)

        @JvmStatic
        private external fun nativeFolderPickCancelled()

        @JvmStatic
        private external fun nativeFolderPickFailed()
    }
}
