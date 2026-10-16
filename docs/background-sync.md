# Connection preview and background runs

New connections default to **Show preview before sync**. This keeps the direction
selection and preflight review flow, including acknowledgement of destructive
Mirror plans. **Don't show preview** makes Run submit work from Connections without
changing pages. It uses the saved allowed directions: upload, download, or both
for Add Only. Mirror and Archive require a single allowed direction when preview
is disabled. The form warns about destructive Mirror operations.

The required `show_preview` boolean is stored in settings and portable connection
lists. There is no configuration migration or missing-field fallback.

Automatic runs still collect inventories and validate a transfer plan. Saved
no-preview policy authorizes the freshly collected plan, including Mirror deletes
and overwrites. Add Only still skips changed files; summaries report the skips.
No-change runs complete with a “No changes were needed” result.

## Runtime ownership

`operations::run_coordinator::RunCoordinator` is the shared entry point for run
admission. It accepts a connection ID for automatic execution, resolves a verified
configuration snapshot, and submits to `BackgroundQueue`. Neither the coordinator
nor the worker requires an AppWindow or a Slint event loop. Preparation happens in
the worker, within the Android background-execution session when applicable.

One operation is admitted at a time, including preparation and queued startup.
Configuration writes cannot change or delete its connection or provider.
Provider credential edits acquire a reservation before touching protected storage;
run admission rejects the provider while that edit is in progress. UI controls
reflect the same busy state, and configuration protection also covers imports and
already-open forms. Other connections and providers remain editable.

Connections displays **Running...** while its operation is pending or active and
**View Summary** after completion, failure, or cancellation. Activity observes the
same queue while navigation remains available. Completion is announced in the app
through the existing desktop notification adapter, and through an Android completion
notification when notification permission is available.

`activity.json`, beside settings, retains display snapshots and results across app
launches. It includes file paths but no credentials, executable requests, or
confirmation tokens. Restored unfinished work becomes a failed/interrupted result;
it is never automatically replayed. Clearing completed Activity also updates this
file. Storage failures are reported, with session results still accessible.

## Android widgets

The launcher offers **SyncPak** (icon, name, connection picker, explanation and
Run) and **SyncPak Compact** (icon, connection picker and Run). Tap the connection
name to configure or change a widget. Each widget remembers its own selection.
Both pickers omit preview-enabled connections and explain why. Unverified entries
can be selected, but their button opens SyncPak for verification instead of syncing.

A Run PendingIntent targets the non-exported foreground service directly. No
Activity is opened. The service submits a connection ID through the same coordinator
as the app, revalidating verification, preview policy and allowed directions. All
widgets disable Run while any connection is preparing or running. The compact
widget replaces its picker text with the running connection's name during a run.
An empty widget provides an Open SyncPak action.

Application owns the JNI host, document-tree adapter, network permission checks,
TLS setup and a process-wide queue. Opening or closing the app screen cannot create
a second queue or cancel a service-owned run. Android initialization uses filesDir
for the settings location, preserving the existing application's files. The local
ndk-context adapter permits both the service/Application and NativeActivity to
register the same process application context; it does not change credential
storage or encryption.

The foreground notification covers preparation and transfers and provides Cancel.
It is removed when the operation ends, and a completion notification opens that
operation's saved results. Notifications require the system's notification permission;
the widget picker requests it on Android 13 and later. File observation and worker
completion callbacks refresh all widget instances when settings or run state change.
Preview-enabled connections remain protected even if a widget's old Run intent is
used after the setting changes. Unfinished work is never automatically replayed.

See [the widget device checklist](android-widgets.md) for launcher and cold-start
validation.
