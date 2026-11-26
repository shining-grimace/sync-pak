This directory contains Slint's Android activity backend from the
`i-slint-backend-android-activity` 1.18.1 crate. See `LICENSES` for its licenses.

The NativeActivity keyboard keeps its composing spans in Android's editable
buffer. Slint receives the visible text and selection and suppresses unchanged
echoes of that state. This avoids rendering the whole composing word as selected
and prevents an echo from replacing a newer keyboard edit.

When Slint moves the caret or changes the text, the bridge restarts input with a
fresh editable buffer. Each connection retains its original buffer, and updates
from retired buffers are ignored. A retired connection can finish its composing
span without affecting the active connection.
