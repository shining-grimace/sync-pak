package com.shininggrimace.syncpak

import android.content.Context
import android.net.Uri
import android.provider.DocumentsContract

/** Readable labels for any Storage Access Framework provider, independent of document ID syntax. */
internal class DocumentTreeLabels(context: Context) {
    private val context = context.applicationContext

    fun label(value: String): String {
        val tree = Uri.parse(value)
        val provider = runCatching {
            val manager = context.packageManager
            tree.authority?.let { manager.resolveContentProvider(it, 0)?.loadLabel(manager) }?.toString()
        }.getOrNull()?.takeIf { it.isNotBlank() } ?: "Document storage"
        val folder = runCatching {
            val document = DocumentsContract.buildDocumentUriUsingTree(
                tree,
                DocumentsContract.getTreeDocumentId(tree),
            )
            context.contentResolver.query(
                document,
                arrayOf(DocumentsContract.Document.COLUMN_DISPLAY_NAME),
                null,
                null,
                null,
            )?.use { cursor ->
                if (cursor.moveToFirst()) cursor.getString(0) else null
            }
        }.getOrNull()?.takeIf { it.isNotBlank() } ?: "Selected folder"
        return "($provider)/$folder"
    }
}
