package com.shininggrimace.syncpak

import android.app.Activity
import android.content.Intent

internal class ConnectionListDocuments(
    private val activity: Activity,
    private val complete: (Int, String) -> Unit,
) {
    private var exportContents: String? = null

    fun pick(contents: String): Int {
        exportContents = contents.takeIf { it.isNotEmpty() }
        activity.runOnUiThread {
            try {
                val intent = if (exportContents == null) {
                    Intent(Intent.ACTION_OPEN_DOCUMENT)
                } else {
                    Intent(Intent.ACTION_CREATE_DOCUMENT).putExtra(Intent.EXTRA_TITLE, "connections.json")
                }
                intent.type = "application/json"
                intent.addCategory(Intent.CATEGORY_OPENABLE)
                activity.startActivityForResult(intent, REQUEST)
            } catch (_: Exception) {
                exportContents = null
                complete(-1, "")
            }
        }
        return 0
    }

    fun result(request: Int, result: Int, data: Intent?): Boolean {
        if (request != REQUEST) return false
        val contents = exportContents
        exportContents = null
        val uri = data?.data
        if (result != Activity.RESULT_OK || uri == null) {
            complete(1, "")
            return true
        }
        Thread {
            try {
                val resolver = activity.contentResolver
                if (contents != null) {
                    val stream = resolver.openOutputStream(uri, "wt") ?: error("No output stream")
                    stream.use { it.write(contents.toByteArray(Charsets.UTF_8)) }
                    complete(0, "")
                } else {
                    val stream = resolver.openInputStream(uri) ?: error("No input stream")
                    val bytes = stream.use { it.readNBytes(4 * 1024 * 1024 + 1) }
                    require(bytes.size <= 4 * 1024 * 1024)
                    val decoder = Charsets.UTF_8.newDecoder()
                    complete(0, decoder.decode(java.nio.ByteBuffer.wrap(bytes)).toString())
                }
            } catch (_: Exception) {
                complete(-1, "")
            }
        }.start()
        return true
    }

    companion object { private const val REQUEST = 4102 }
}
