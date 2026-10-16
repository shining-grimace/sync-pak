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

## Future Android widget

The widget should pass a connection ID to a service-owned runtime and use this same
coordinator and queue in the application's default process. It must not construct
a second queue when the app runtime is already running. Preview-enabled connections
are rejected by `start_automatic`; they require opening the interactive flow.

The widget and cold-start service entry point are intentionally deferred. Android
JNI adapters for document trees, networking, and foreground notifications currently
initialize through the Activity. A service host will need to initialize these from
an application/service context, along with credentials and TLS, without creating
an Activity. Keep platform initialization separate from operation admission. The
service must retain the runtime independently of the window, provide Android
notifications/cancellation, and respect platform execution limits. Persisted results
allow Activity to display a service run's outcome when the UI opens later.
