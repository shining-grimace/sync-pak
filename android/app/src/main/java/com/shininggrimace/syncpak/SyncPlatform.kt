package com.shininggrimace.syncpak

import android.Manifest
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager

/** JNI host retained for the process, containing no Activity or screen references. */
class SyncPlatform(context: Context) {
    private val context = context.applicationContext
    private val documents = DocumentTreeAccess(this.context)
    fun applicationContext(): Context = context
    fun hasInternetPermission(): Boolean = context.checkSelfPermission(Manifest.permission.INTERNET) == PackageManager.PERMISSION_GRANTED
    fun verifyFolder(uri: String): Int = documents.verify(uri)
    fun inventoryFolder(uri: String): String? = documents.inventory(uri)
    fun folderEntryMetadata(uri: String, path: String): String? = documents.metadata(uri, path)
    fun openFolderFileForRead(uri: String, path: String): Int = documents.openForRead(uri, path)
    fun openFolderFileForWrite(uri: String, path: String): Int = documents.openForWrite(uri, path)
    fun createFolderPath(uri: String, path: String): Int = documents.createDirectories(uri, path)
    fun deleteFolderEntry(uri: String, path: String): Int = documents.delete(uri, path)
    fun startSyncExecution(name: String) {
        context.startForegroundService(SyncExecutionService.startIntent(context, name))
        SyncWidgets.refresh(context)
    }
    fun updateSyncExecution(name: String, current: Int, total: Int) = SyncExecutionNotification.update(context, name, current, total)
    fun finishSyncOperation(name: String, summary: String, operationId: String) {
        stopSyncExecution()
        SyncExecutionNotification.complete(context, name, summary, operationId)
    }
    fun peekRequestedResult(): String = SyncResultNavigation.pending.orEmpty()
    fun clearRequestedResult(id: String) { SyncResultNavigation.clear(id) }
    fun finishSyncExecution(name: String, summary: String) {
        stopSyncExecution()
        SyncExecutionNotification.complete(context, name, summary)
    }
    fun stopSyncExecution() {
        context.stopService(Intent(context, SyncExecutionService::class.java))
        SyncExecutionNotification.clearRunning(context)
    }
    fun widgetsChanged() { SyncWidgets.refresh(context) }
}
