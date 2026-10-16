package com.shininggrimace.syncpak

/** All entry points use the process-owned Rust queue; none create a NativeActivity. */
object SyncNative {
    init { System.loadLibrary("sync_pak") }
    @JvmStatic external fun initialize(host: SyncPlatform, files: String)
    @JvmStatic external fun snapshot(): String?
    @JvmStatic external fun start(connectionId: String): String?
    fun state(): org.json.JSONObject = runCatching {
        snapshot()?.let { org.json.JSONObject(it) }
            ?: org.json.JSONObject().put("error", "Connections unavailable. Open SyncPak to check settings.")
    }.getOrElse { org.json.JSONObject().put("error", "Connections unavailable. Open SyncPak to check settings.") }
}
