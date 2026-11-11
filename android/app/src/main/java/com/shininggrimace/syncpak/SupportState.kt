package com.shininggrimace.syncpak

/** Values are shared with settings-support.slint; ownership is monotonic on this device. */
internal data class SupportState(
    val phase: Int = LOADING,
    val price: String = "",
    val message: String = "",
) {
    fun transition(next: SupportState): SupportState = if (phase == OWNED) this else next

    companion object {
        const val LOADING = 0
        const val READY = 1
        const val PROCESSING = 2
        const val PENDING = 3
        const val OWNED = 4
        const val ERROR = 5
        const val UNCONFIGURED = 6
    }
}
