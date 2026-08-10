package com.shininggrimace.syncpak

import android.content.Context
import android.net.Uri
import android.provider.DocumentsContract
import org.json.JSONArray
import org.json.JSONObject

internal class DocumentTreeAccess(context: Context) {
    private val resolver = context.applicationContext.contentResolver

    fun verify(value: String): Int = safely {
        val tree = Uri.parse(value)
        val permission = resolver.persistedUriPermissions.firstOrNull { it.uri == tree }
            ?: return@safely MISSING_PERMISSION
        if (!permission.isReadPermission || !permission.isWritePermission) {
            return@safely MISSING_PERMISSION
        }
        val root = root(tree) ?: return@safely NOT_FOUND
        if (root.mimeType == DocumentsContract.Document.MIME_TYPE_DIR) OK else NOT_DIRECTORY
    }

    fun inventory(value: String): String? = safely(null) {
        val tree = Uri.parse(value)
        val root = root(tree) ?: return@safely null
        if (root.mimeType != DocumentsContract.Document.MIME_TYPE_DIR) return@safely null
        val result = JSONArray()
        val pending = ArrayDeque<Pair<Document, String>>()
        val visited = mutableSetOf(root.id)
        pending.add(root to "")
        while (pending.isNotEmpty()) {
            val (parent, prefix) = pending.removeFirst()
            for (child in children(tree, parent)) {
                require(isValidComponent(child.name)) { "The provider returned an invalid name" }
                val path = if (prefix.isEmpty()) child.name else "$prefix/${child.name}"
                result.put(child.toJson(path))
                if (child.mimeType == DocumentsContract.Document.MIME_TYPE_DIR &&
                    visited.add(child.id)
                ) {
                    pending.add(child to path)
                }
            }
        }
        result.toString()
    }

    fun metadata(value: String, path: String): String? = safely(null) {
        resolve(Uri.parse(value), path)?.toJson(path)?.toString()
    }

    fun openForRead(value: String, path: String): Int = safely {
        val document = resolve(Uri.parse(value), path) ?: return@safely NOT_FOUND
        if (document.mimeType == DocumentsContract.Document.MIME_TYPE_DIR) {
            return@safely NOT_FILE
        }
        resolver.openFileDescriptor(document.uri, "r")?.detachFd() ?: UNAVAILABLE
    }

    fun openForWrite(value: String, path: String): Int = safely {
        val tree = Uri.parse(value)
        val (parentPath, name) = splitParent(path) ?: return@safely INVALID_PATH
        val parent = ensureDirectories(tree, parentPath) ?: return@safely UNAVAILABLE
        val existing = child(tree, parent, name)
        if (existing?.mimeType == DocumentsContract.Document.MIME_TYPE_DIR) {
            return@safely NOT_FILE
        }
        val document = existing ?: create(tree, parent, "application/octet-stream", name)
            ?: return@safely UNAVAILABLE
        resolver.openFileDescriptor(document.uri, "wt")?.detachFd() ?: UNAVAILABLE
    }

    fun createDirectories(value: String, path: String): Int = safely {
        if (ensureDirectories(Uri.parse(value), path) == null) UNAVAILABLE else OK
    }

    fun delete(value: String, path: String): Int = safely {
        val document = resolve(Uri.parse(value), path) ?: return@safely OK
        if (DocumentsContract.deleteDocument(resolver, document.uri)) OK else UNAVAILABLE
    }

    private fun root(tree: Uri): Document? {
        val id = DocumentsContract.getTreeDocumentId(tree)
        return query(DocumentsContract.buildDocumentUriUsingTree(tree, id))
    }

    private fun resolve(tree: Uri, path: String): Document? {
        var current = root(tree) ?: return null
        for (name in components(path) ?: return null) {
            current = child(tree, current, name) ?: return null
        }
        return current
    }

    private fun ensureDirectories(tree: Uri, path: String): Document? {
        var current = root(tree) ?: return null
        for (name in components(path) ?: return null) {
            val existing = child(tree, current, name)
            current = when {
                existing == null -> create(
                    tree,
                    current,
                    DocumentsContract.Document.MIME_TYPE_DIR,
                    name,
                )
                existing.mimeType == DocumentsContract.Document.MIME_TYPE_DIR -> existing
                else -> return null
            } ?: return null
        }
        return current
    }

    private fun child(tree: Uri, parent: Document, name: String): Document? =
        children(tree, parent).firstOrNull { it.name == name }

    private fun children(tree: Uri, parent: Document): List<Document> {
        val uri = DocumentsContract.buildChildDocumentsUriUsingTree(tree, parent.id)
        val documents = mutableListOf<Document>()
        resolver.query(uri, PROJECTION, null, null, null)?.use { cursor ->
            while (cursor.moveToNext()) {
                document(tree, cursor)?.let(documents::add)
            }
        } ?: throw IllegalStateException("The document provider returned no cursor")
        return documents
    }

    private fun query(uri: Uri): Document? =
        resolver.query(uri, PROJECTION, null, null, null)?.use { cursor ->
            if (cursor.moveToFirst()) document(uri, cursor) else null
        }

    private fun document(tree: Uri, cursor: android.database.Cursor): Document? {
        val id = cursor.getString(cursor.getColumnIndexOrThrow(DocumentsContract.Document.COLUMN_DOCUMENT_ID))
            ?: return null
        val name = cursor.getString(cursor.getColumnIndexOrThrow(DocumentsContract.Document.COLUMN_DISPLAY_NAME))
            ?: return null
        val mimeType = cursor.getString(cursor.getColumnIndexOrThrow(DocumentsContract.Document.COLUMN_MIME_TYPE))
            ?: return null
        return Document(
            id,
            name,
            mimeType,
            cursor.optionalLong(DocumentsContract.Document.COLUMN_SIZE)?.coerceAtLeast(0) ?: 0,
            cursor.optionalLong(DocumentsContract.Document.COLUMN_LAST_MODIFIED)?.takeIf { it > 0 },
            DocumentsContract.buildDocumentUriUsingTree(tree, id),
        )
    }

    private fun create(tree: Uri, parent: Document, mimeType: String, name: String): Document? {
        val uri = DocumentsContract.createDocument(resolver, parent.uri, mimeType, name) ?: return null
        val document = query(uri) ?: return null
        if (document.name != name) {
            DocumentsContract.deleteDocument(resolver, uri)
            return null
        }
        return document.copy(
            uri = DocumentsContract.buildDocumentUriUsingTree(tree, DocumentsContract.getDocumentId(uri)),
        )
    }

    private fun components(path: String): List<String>? {
        if (path.isEmpty()) return emptyList()
        val result = path.split('/')
        return result.takeIf { parts ->
            parts.all(::isValidComponent)
        }
    }

    private fun isValidComponent(value: String): Boolean =
        value.isNotEmpty() && value != "." && value != ".." &&
            !value.contains('/') && !value.contains('\\')

    private fun splitParent(path: String): Pair<String, String>? {
        val parts = components(path)?.takeIf { it.isNotEmpty() } ?: return null
        return parts.dropLast(1).joinToString("/") to parts.last()
    }

    private inline fun safely(action: () -> Int): Int = try {
        action()
    } catch (_: SecurityException) {
        MISSING_PERMISSION
    } catch (_: Exception) {
        UNAVAILABLE
    }

    private inline fun <T> safely(fallback: T, action: () -> T): T = try {
        action()
    } catch (_: Exception) {
        fallback
    }

    private data class Document(
        val id: String,
        val name: String,
        val mimeType: String,
        val size: Long,
        val modifiedMilliseconds: Long?,
        val uri: Uri,
    ) {
        fun toJson(path: String): JSONObject = JSONObject()
            .put("path", path)
            .put("kind", if (mimeType == DocumentsContract.Document.MIME_TYPE_DIR) "directory" else "file")
            .put("size", size)
            .put("modified", modifiedMilliseconds?.div(1000) ?: JSONObject.NULL)
    }

    private fun android.database.Cursor.optionalLong(column: String): Long? {
        val index = getColumnIndex(column)
        return if (index < 0 || isNull(index)) null else getLong(index)
    }

    private companion object {
        const val OK = 0
        const val NOT_FOUND = -2
        const val NOT_DIRECTORY = -3
        const val NOT_FILE = -4
        const val MISSING_PERMISSION = -5
        const val INVALID_PATH = -6
        const val UNAVAILABLE = -1

        val PROJECTION = arrayOf(
            DocumentsContract.Document.COLUMN_DOCUMENT_ID,
            DocumentsContract.Document.COLUMN_DISPLAY_NAME,
            DocumentsContract.Document.COLUMN_MIME_TYPE,
            DocumentsContract.Document.COLUMN_SIZE,
            DocumentsContract.Document.COLUMN_LAST_MODIFIED,
        )
    }
}
