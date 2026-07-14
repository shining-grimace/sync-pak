
# App Overview

This app, with stylised name "SyncPak", is a GUI tool for synchronising local directories with cloud providers.

Vision: Create a free-to-use, no-nonsense, privacy-focused tool that's configured once and then syncs directories between places effortlessly.

Target OSs (all supported in first release):
- Linux
- Android
- Windows

Supported cloud providers:
- Cloudflare R2 (using the S3-compatible API under the hood)
- Backblaze B2
- AWS S3

Intended distribution:
- Snap (for Ubuntu)
- Flathub (for other Linux distros)
- Google Play (for Android)
- A signed MSIX package through the Microsoft Store on Windows, with the same signed
  package also available for direct installation where practical. MSIX provides a
  clean install/uninstall model, package identity, and support for Windows platform
  features such as notifications and background tasks. See the
  [Microsoft packaging overview](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/packaging/).

Code:
- Rust and the Slint framework
- All open-source

Monetisation:
- The Android app to be monetised with ads through Google AdMob

# Design

Theming:
- Model the UI structure on the WSL Dashboard app
- Use the same palette as Stitchy Android by Shining Grimace

Presentation:
- Easy to use for anyone
- Include a first-use welcome screen
- Transparent for privacy-minded people
- Heavy on explanation text
- Focused on visual layouts

Language and Accessibility:
- UI should be designed to be high-contrast for accessibility
- Full Unicode (UTF-8) support should be throughout (and attempting to use a path not supported by UTF-8 should fail)

Storage model:
- Most settings and configurations should be stored in a single JSON file (use `serde_json` when working with it)
- The config file should be stored according to XDG standards on Linux, or app-specific directories on Android, and whatever the best practice is on Windows
- Secure credentials (cloud provider credentials config, as JSON) should be stored in a platform-backed credential facility

## Credential Management

The ordinary configuration file stores a stable provider ID, a user-visible provider
name, and the provider type, but no access keys, secret keys, session tokens, or other
secret values. The provider ID is used to retrieve the corresponding credential JSON
from platform-backed secure storage:

- Linux uses a Secret Service-compatible keyring available within the Snap or Flatpak
  sandbox.
- Android generates a non-exportable encryption key in Android Keystore and stores only
  ciphertext in the app's private storage.
- Windows uses a credential facility associated with the packaged application identity.

There must be no plaintext-file fallback. If secure storage is locked or unavailable,
SyncPak should explain the problem and leave affected providers unavailable until the
facility can be used. Creating or editing a provider must save the secure credential
before committing its non-secret configuration; failure must leave the previous version
intact. Deleting a provider must remove both records after its dependent operations have
been cancelled.

Credentials must never be included in logs, error details, analytics, clipboard content
created without an explicit user action, or configuration exports. Credential form fields
are obscured by default and may be revealed temporarily by the user. Secrets should be
kept in memory only while required for an operation and should not be retained by queue
history entries.

## Privacy Presentation

The welcome screen and a permanent Privacy page should explain, in plain language:

- Files travel directly between the user's device and the selected cloud provider;
  SyncPak does not operate an intermediary file service.
- Provider credentials are stored using the operating system's protected facilities and
  are sent only to the selected provider.
- Which configuration and transient operation information is stored locally, and that
  the in-memory activity list is discarded when the app exits.
- Whether crash reporting or analytics are present. They must be opt-in if introduced,
  and the UI must never claim that no telemetry exists if a release includes it.
- On Android, that AdMob may process device or advertising data. Required consent and
  privacy controls must be presented before ads are initialized. Ads must not appear on
  provider forms, path selectors, operation previews, progress/results views, or any
  screen displaying credentials or file names.
- Files are not encrypted by SyncPak before transfer; provider-side or transport
  encryption should not be described as end-to-end encryption.

Permission prompts should be preceded by an explanation of why access is needed and what
will be accessible. Destructive mirror previews must show exactly which files will be
overwritten or deleted. Logs and shareable diagnostics must redact credentials and avoid
file paths by default, with an explicit warning before the user includes path details.
The Privacy page should link to the published privacy policy, open-source code, licences,
and relevant provider privacy policies.

## Sync Connection Presentation

Connections are displayed as responsive cards. Each card uses a simple endpoint diagram:
the local folder on one side, a directional arrow in the middle, and the provider, bucket,
and remote path on the other. Archive cards instead use an archive-box symbol and show the
retention value. The connection name is the primary heading, followed by a text mode badge
(`Read-only`, `Mirror`, or `Archive`) and any current queue status.

Long paths use middle truncation visually while exposing the complete value through an
accessible label and a copy action. Mode, direction, and status must never be communicated
by colour alone. Cards provide clearly labelled Run, Edit, and More actions; destructive
Delete is inside More. A running or queued card links to its activity entry rather than
allowing a duplicate run. Desktop layouts use a grid where space permits, while narrow and
touch layouts use a single-column list with at least platform-standard touch target sizes.

# Provider Configs

A section of the app is dedicated to listing, adding, editing, verifying, and deleting the saved provider configs.

Provider config should:
- Focus on important, required fields and hide advanced fields which would rarely get used

Provider configs are stored in secure platform-managed storage.

If a provider is edited, nothing should be done to change the associated sync connections automatically. Trying to delete a provider should ask the user for confirmation, and list which sync connections are associated with the provider, noting that they'll be deleted as well.

# Sync Connections

A section of the app is dedicated to listing, adding, editing, running, and deleting sync connections. Sync operations are run manually from a GUI; there will be no scheduling feature.

A sync connection config stores:
- A user-friendly name
- A remote provider paired with a directory path
- A local directory path
- Mode: read-only, mirror (stateless), or archive
- (For archives only) Keep last N archives for this connection (defaults to 1, required to be at least 1)
- The remote provider bucket name (if possible, let the user select it from a list, but allow that the fetching of the list of buckets might be forbidden using the provided credentials, so use free-text input when the select method is unusable)

The local directory path can be selected from a filepicker interface, or edited in a free-text input.

When sync operations are run, missing directories will be created as needed in the target location.

Connection configs are allowed to overlap (share the same providers, remote paths, or local paths).

Read-only mode copies new files from source to destination without overwriting or deleting
anything. It can run as upload, download, or additive both ways. In both-ways mode, paths
which exist on only one side are copied to the other side; paths which differ are skipped
with warnings. Files which exist only at the destination are not noteworthy.

If there are filesystem errors when running an operation, the precise error should be shown in the UI.

## File Comparison

Every operation begins with a non-mutating inventory and preflight. Paths are compared as
case-sensitive, normalized relative paths. A regular file is treated as unchanged when it
has the same type, byte size, and modification time on both sides. Modification times are
compared at the coarsest precision supported by the two endpoints, with a tolerance of up
to two seconds for filesystems with low timestamp precision. For remote objects, use a
source modification time previously written by SyncPak when available, and otherwise use
the provider's recorded last-modified time. During a copy, SyncPak preserves the source
modification time as a local file time or remote object metadata where the destination
supports it.

If either modification time is unavailable, files of the same size are treated as changed
rather than assumed equal. Read-only mode skips such files with a warning, while mirror
mode includes them in the overwrite preview. For symlinks, the link target text is compared
directly. Directory metadata and permissions are compared separately and handled on a
best-effort basis.

SyncPak does not calculate content hashes or download remote files merely to compare them.
This keeps preflight quick and avoids unexpected bandwidth use, at the accepted cost that
two files with the same size and modification time may be incorrectly treated as unchanged.

The UI uses the terms `New`, `Unchanged`, `Changed`, `Will overwrite`, `Will delete`,
`Unsupported`, and `Warning`. Technical size and timestamp details belong in an expandable
detail area rather than the primary file list.

Archive mode creates a ZIP of the source directory and stores it at the destination: on the
local machine when downloading, or in the cloud when uploading. The filename is a UTC
timestamp formatted as `YYYYMMDD-HHMMSSZ`, followed by a space, the user-friendly
connection name, and `.zip`. Creating an archive fails if that exact filename already
exists. After an archive is uploaded successfully, its temporary local copy is deleted.

Archive filenames preserve non-ASCII text as UTF-8 normalized to NFC. To make a single
portable filename segment, control characters and `/`, `\`, `<`, `>`, `:`, `"`, `|`, `?`,
and `*` are replaced with `_`; trailing spaces and periods are removed. If nothing remains,
`Archive` is used. The sanitized connection-name portion is truncated on a UTF-8 character
boundary as needed to keep the complete filename within 200 UTF-8 bytes. ZIP entry names
are stored with the ZIP UTF-8 flag. A non-UTF-8 source path is a fatal preflight error and
must never be converted lossily.

Archive zips are only deleted as per settings after one has successfully saved. The keep-last-N policy applies to both local and remote archives.

To initiate a sync, the user chooses the direction: upload, download, or both ways. Both ways is only available for read-only mode connections.

The sync operation occurs while a modal UI is shown; this has a loading indicator as needed, and will list files that are done or in progress, leaving the UI shown at the end so the user can browse the list of affected files before dismissing the modal.

In mirror mode, if there are any deletes or overwrites that would happen, these need to be listed in the modal UI and require confirmation before the sync starts.

Hidden files, empty directories, symlinks, and filesystem permissions are to be included in all sync operations. These will be best-effort, and warnings will be given to the user if they cannot be handled properly due to platform-specific or provider-specific behaviours. Symlinks will be copied as the links themselves (not the underlying file).

File names are case-sensitive (differences on case are considered separate files).

Failures to resolve things due to casing issues on the host platform or issues around UTF-8 support should be considered fatal issues that block the sync from being started.

Sync connections cannot run concurrently, but there should be a queue of them which can be viewed as a list in the UI somewhere. While running, the app should keep this list of sync operations, with status showing as queued, failed, completed, cancelled, or in progress. Since it is only kept in memory, it does not persist across app launches. Each activity entry stores an immutable, non-secret snapshot of the connection name, mode, direction, endpoints, and result so completed entries remain understandable after configuration changes. Deleting a connection or provider cancels its in-progress operation and removes its queued operations before deleting configuration or credentials.

In progress connections can be cancelled from the queue list or from a notification-like popup UI at the bottom of the app UI, functioning kind of like a snackbar to show what's in progress.

On Android, if the app is sent to the background while a sync is running, the sync needs to keep running to completion and show a persistent notification during that time.

## Failure and Recovery

Planning and preflight must complete before any destination is changed. Fatal path,
permission, credential, bucket, capacity, casing, or encoding errors block the operation
and are presented together where possible. Mirror confirmation is based on the completed
plan; if either side changes before execution, the plan is invalidated and preflight and
confirmation run again.

Transfers follow these rules:

- Local downloads are written to a uniquely named temporary file in the destination
  directory, flushed, and atomically renamed into place where the filesystem permits.
  Existing destinations remain intact until replacement succeeds.
- Cloud uploads use provider-supported multipart upload for large objects. Incomplete
  multipart uploads are aborted on failure or cancellation. The final object is considered
  complete after the provider confirms that the upload completed successfully.
- Copies and overwrites complete before mirror deletions begin. A failed copy prevents the
  deletion phase, reducing the chance that a partial run removes the only good copy.
- Retention pruning occurs only after a new archive is stored successfully. It applies only
  to archives belonging to that connection. A configured keep-last-N value is advance
  authorization to remove older archives without another confirmation.
- A missing source is fatal. Missing directories are created only on the destination.

Transient network failures, throttling, and retryable provider errors use bounded
exponential backoff with jitter. Make at most four attempts in total, while respecting a
provider-supplied retry delay. Authentication failures, permission failures, invalid
requests, insufficient local space, and deterministic filesystem errors are not retried.
Retry status and the next attempt are shown in the operation UI.

Cancellation stops scheduling new files, aborts safely abortable transfers, and never
rolls back completed work. The result is `Cancelled` and lists completed, incomplete, and
not-started items. Closing the progress modal does not cancel the operation; it minimizes
to the activity snackbar. On Android, an active operation is owned by the foreground
service represented by the persistent notification. If the process is nevertheless
terminated, SyncPak does not claim the run succeeded and does not automatically resume it.

Queues and results are intentionally not crash-persistent. On startup, SyncPak removes
stale temporary files and aborts identifiable stale multipart uploads where supported,
without touching final destination files. Cleanup failures are reported as warnings.
Every error contains a plain-language summary, the affected path or provider operation,
a retry action where useful, and expandable redacted technical details.

# UI Designs

The application shell uses a sidebar on wide desktop windows and compact bottom or drawer
navigation on narrow and Android layouts. Primary destinations are Connections, Providers,
Activity, and Privacy & About. The currently active operation remains reachable from a
snackbar and, on Android, the system notification.

## Welcome

Shown on first use. It explains the local-to-provider model, credential storage, Android
advertising disclosure where applicable, and the difference between read-only, mirror,
and archive modes. Its primary action creates a provider; a secondary action opens Privacy
& About. It should not request permissions before explaining them.

## Connections List

Displays the responsive connection cards described above, with filters for All, Read-only,
Mirror, and Archive. The empty state guides the user to add a provider first or create a
connection when a provider exists. Run opens the direction dialog; Edit opens the
connection form; Delete opens a confirmation dialog. Queued and running connections open
their Activity entry.

## Create/Edit Connection

A single form contains name, mode, provider, bucket, remote path, and local path. Archive
mode additionally shows Keep last N. The mode selector immediately updates a persistent
plain-language explanation and hides irrelevant fields. The local path supports a native
picker and direct entry. Bucket selection attempts to load available buckets, then offers
manual entry if listing is unavailable. Save validates all fields but performs no transfer.
Leaving with changes prompts to discard or continue editing.

## Providers List

Shows provider name, type, verification state for the current session, and the number of
connections using it. Actions are Verify, Edit, and Delete. The empty state explains that
credentials remain on the device and offers Add provider.

## Create/Edit Provider

The form first selects Cloudflare R2, Backblaze B2, or AWS S3, then shows only required
fields. Advanced fields such as a custom endpoint or temporary session token are collapsed
under Advanced. Secret fields are obscured and have a temporary reveal control. Verify
tests authentication and basic provider access without modifying cloud data. Save and
Verify is the primary action; Save without verification remains available with a warning.

## Delete Provider Confirmation

Names the provider and lists every dependent connection. Confirmation states that queued
or running operations will be cancelled, dependent connections will be deleted, and cloud
files will not be touched. The destructive action repeats the provider name.

## Run Direction

Shows Upload and Download for every mode, plus Both ways only for read-only mode. Each
choice depicts source and destination explicitly. For archive mode it also shows the
resulting ZIP destination and retention policy. Continue starts inventory and preflight;
it does not yet modify files.

## Preflight and Mirror Confirmation

Shows inventory progress followed by a categorized plan. Fatal issues prevent
starting. Read-only and archive operations can start when preflight succeeds. Mirror plans
separately list additions, overwrites, and deletions with counts, sizes, expandable paths,
and a required confirmation checkbox. If there are no destructive actions, the mirror can
start without the destructive confirmation.

## Operation Progress and Result

The modal has overall progress, current phase, transferred bytes, and a virtualized list
of file statuses. Cancel is always available while work is active. Minimize leaves the run
active and exposes it through the snackbar and Activity screen. At completion the modal
remains open and summarizes completed, skipped, warning, failed, and deleted items. Failed
and cancelled results offer Retry as a new preflighted run, not a blind continuation.

## Activity

Lists the in-memory queue and this-launch results in newest-first order. Queued entries can
be removed; the active entry can be cancelled; completed entries expose their result
details. `Clear completed` removes completed, failed, and cancelled entries only. A note
explains that activity history is cleared when SyncPak exits.

## Privacy & About

Contains the privacy presentation defined above, version and licence information, links to
source code and policies, and Android consent controls where required. A diagnostics area
can copy redacted app/version information and optionally include paths only after a warning.

## Common Dialogs and States

The UI includes discard-changes confirmation, delete-connection confirmation, cancel-run
confirmation, permission rationale, secure-storage-unavailable, offline, empty, loading,
and unexpected-error states. Dialog focus is trapped correctly, Escape/Back behaves
consistently, and focus returns to the invoking control. Lists and progress updates must be
usable with keyboard and screen readers and must not rely on animation or colour alone.

# Copy

Text below is source copy. Placeholders use braces, for example `{provider}`. Provider or
operating-system errors may be appended under `Technical details`, but must not replace the
plain-language message.

## Common

- Actions: `Add`, `Back`, `Cancel`, `Close`, `Continue`, `Copy`, `Delete`, `Discard`,
  `Done`, `Edit`, `Minimize`, `More`, `Remove from queue`, `Retry`, `Reveal`, `Save`,
  `Save and verify`, `Select folder`, `Start`, `Verify`.
- Statuses: `Not verified`, `Verified`, `Checking`, `Queued`, `In progress`, `Retrying`,
  `Completed`, `Completed with warnings`, `Failed`, `Cancelled`.
- File states: `New`, `Unchanged`, `Changed`, `Will overwrite`, `Will delete`, `Skipped`,
  `Unsupported`, `Warning`, `Failed`, `Complete`, `Not started`.
- Generic validation: `This field is required.`, `Enter a whole number of at least 1.`,
  `Choose a folder.`, `Choose a provider.`, `Enter a bucket name.`

## Welcome

- Heading: `Your files, your cloud.`
- Body: `SyncPak copies folders directly between this device and a cloud provider you
  choose. SyncPak does not operate an intermediary file service.`
- Credential note: `Your provider credentials are protected using this device's secure
  storage and are sent only to that provider.`
- Modes heading: `Choose how each connection behaves`
- Read-only summary: `Copy new files without overwriting or deleting anything.`
- Mirror summary: `Make a destination match its source after previewing destructive changes.`
- Archive summary: `Create timestamped ZIP archives and keep the number you choose.`
- Primary action: `Add your first provider`
- Secondary action: `Read how SyncPak protects your privacy`
- Android ad note: `The Android version is supported by ads. Ads are kept away from screens
  that show credentials, paths, filenames, or transfer activity.`

## Connections

- Title: `Connections`
- Intro: `A connection links a local folder with a folder in one cloud bucket.`
- Primary action: `New connection`
- Empty heading: `No connections yet`
- Empty body without providers: `Add a cloud provider, then create a connection to choose
  what to copy.`
- Empty body with providers: `Create a connection to link a local folder with cloud storage.`
- Filters: `All`, `Read-only`, `Mirror`, `Archive`
- Card endpoint labels: `On this device`, `In {provider}`
- Archive retention: `Keeps the last {count} archives`
- Active action: `View activity`
- Delete title: `Delete {connection}?`
- Delete body: `This removes the connection from SyncPak. It does not delete local or cloud
  files. Queued or running operations for this connection will be cancelled.`
- Delete action: `Delete connection`

## Connection Form

- New title: `New connection`
- Edit title: `Edit {connection}`
- Fields: `Connection name`, `Mode`, `Provider`, `Bucket`, `Remote folder`, `Local folder`,
  `Keep last N archives`
- Name help: `Used in the connection list and archive filenames.`
- Remote help: `Leave empty to use the root of the bucket.`
- Retention help: `After a new archive is saved successfully, older archives made by this
  connection are deleted until this many remain.`
- Read-only description: `Copies files that exist only at the source. Existing changed
  files are skipped, and nothing is overwritten or deleted.`
- Mirror description: `Makes the destination an exact copy of the source. Overwrites and
  deletions are always shown for confirmation before the run starts.`
- Archive description: `Creates a timestamped ZIP from the source and stores it at the
  destination. Older archives are removed according to this connection's retention setting.`
- Bucket loading: `Loading buckets…`
- Bucket unavailable: `SyncPak could not list buckets with these credentials. Enter the
  bucket name instead.`
- Unsaved title: `Discard unsaved changes?`
- Unsaved body: `Your changes to this connection have not been saved.`
- Saved: `Connection saved.`

## Providers

- Title: `Providers`
- Intro: `Provider credentials are kept in this device's protected storage.`
- Primary action: `Add provider`
- Empty heading: `No providers yet`
- Empty body: `Add credentials for Cloudflare R2, Backblaze B2, or AWS S3.`
- Usage count: `{count} connections`
- Verification success: `Connected to {provider}.`
- Verification failure: `SyncPak could not verify {provider}. Check the fields and technical
  details, then try again.`
- Delete title: `Delete {provider}?`
- Delete body: `This deletes its protected credentials and the connections listed below.
  Their queued or running operations will be cancelled. No cloud files will be deleted.`
- Delete action: `Delete provider and {count} connections`
- Secure storage unavailable: `Protected credential storage is unavailable. Unlock or
  enable your device's credential service, then try again.`

## Provider Form

- New title: `Add provider`
- Edit title: `Edit {provider}`
- Common fields: `Provider name`, `Provider type`, `Access key ID`, `Secret access key`
- Cloudflare R2 fields: `Account ID`, `Access key ID`, `Secret access key`
- Backblaze B2 fields: `Key ID`, `Application key`
- AWS S3 fields: `Access key ID`, `Secret access key`, `Region`
- Advanced heading: `Advanced`
- Advanced fields where supported: `Custom endpoint`, `Session token`
- Reveal warning: `Make sure nobody else can see your screen.`
- Verify explanation: `Verification signs in and checks basic access. It does not create,
  change, or delete cloud files.`
- Save unverified title: `Save without verifying?`
- Save unverified body: `This provider may not work until its credentials and access are
  verified.`
- Saved: `Provider saved securely.`

## Run Direction and Preflight

- Title: `Run {connection}`
- Upload: `Upload`
- Upload help: `Use the local folder as the source and cloud storage as the destination.`
- Download: `Download`
- Download help: `Use cloud storage as the source and the local folder as the destination.`
- Both ways: `Both ways`
- Both-ways help: `Copy files found on only one side to the other. Changed files are skipped.`
- Preflight title: `Checking before anything changes`
- Listing: `Listing files…`
- No mutation note: `No files are changed during this check.`
- Ready: `Ready to start`
- Fatal heading: `This operation cannot start`
- Fatal body: `Resolve the issues below, then run the connection again. No files were changed.`
- Mirror title: `Review mirror changes`
- Mirror warning: `The destination will be made to match the source. Review every overwrite
  and deletion before continuing.`
- Mirror checkbox: `I understand that the listed destination files will be overwritten or deleted.`
- Counts: `{additions} new`, `{overwrites} overwrites`, `{deletions} deletions`, `{skipped} skipped`
- Start actions: `Start upload`, `Start download`, `Create archive`, `Start mirror`

## Progress, Queue, and Results

- Activity title: `Activity`
- Empty heading: `No activity this time`
- Empty body: `Queued and completed operations appear here until SyncPak exits.`
- Phases: `Preparing`, `Copying`, `Finalizing`, `Deleting`, `Pruning old archives`, `Cleaning up`
- Progress: `{completed} of {total} items · {transferred} of {total_size}`
- Retry status: `Trying again in {duration} ({attempt} of 4)`
- Minimize note: `The operation will continue in Activity.`
- Cancel title: `Cancel this operation?`
- Cancel body: `Completed changes will not be undone. Work that has not started will be skipped.`
- Cancel action: `Cancel operation`
- Result success: `{connection} completed successfully.`
- Result warning: `{connection} completed with {count} warnings.`
- Result failure: `{connection} failed. Completed changes were not undone.`
- Result cancelled: `{connection} was cancelled. Completed changes were not undone.`
- Clear action: `Clear completed`
- Clear explanation: `Removes completed, failed, and cancelled entries from this activity list.`
- Android notification channel: `Sync operations`
- Android notification running: `SyncPak is running {connection}`
- Android notification queued: `{count} operations waiting`
- Android notification cancel: `Cancel`

## Archive Messages

- Creating: `Creating {filename}…`
- Downloading source: `Downloading files for the archive…`
- Uploading: `Uploading archive…`
- Pruning: `Removing archives older than the newest {count}…`
- Collision: `An archive named {filename} already exists. No archive was replaced.`
- Temporary retained: `The upload did not complete. The temporary archive was kept for
  cleanup or retry.`

## Errors and Warnings

- Missing source: `The source {path} does not exist. SyncPak creates missing destinations,
  but never creates a missing source.`
- Unsupported path: `{path} cannot be represented safely as UTF-8.`
- Case collision: `These paths differ only by letter case, but the destination cannot store
  both: {paths}.`
- Permission denied: `SyncPak does not have permission to access {path}.`
- Local space: `There is not enough free space to complete this operation.`
- Offline: `The provider could not be reached. Check your connection and try again.`
- Authentication: `{provider} rejected the saved credentials. Edit or replace them, then verify again.`
- Provider permission: `{provider} did not allow {operation} in {bucket}. Check the key's permissions.`
- Changed skip: `{path} exists on both sides with different contents and was not changed.`
- Unsupported metadata: `{attribute} could not be preserved for {path} on this destination.`
- Plan changed: `Files changed after the preview was prepared. SyncPak will check again before starting.`
- Cleanup warning: `SyncPak could not remove temporary data from an earlier operation.`
- Technical disclosure: `Technical details are redacted where they may contain credentials.`

## Privacy & About

- Title: `Privacy & About`
- Direct transfer heading: `Direct transfers`
- Direct transfer body: `Your files move directly between this device and the cloud provider
  you configured. SyncPak does not receive or host them.`
- Credentials heading: `Protected credentials`
- Credentials body: `Provider credentials are stored using this device's protected facilities
  and are retrieved only when SyncPak connects to that provider.`
- Local data heading: `Data on this device`
- Local data body: `Connection settings are stored locally. Activity is kept in memory and
  cleared when SyncPak exits.`
- Encryption heading: `File encryption`
- Encryption body: `SyncPak does not encrypt file contents before transfer. Your provider and
  HTTPS connection may provide other forms of encryption.`
- Diagnostics action: `Copy redacted diagnostics`
- Include-paths option: `Include file and folder paths`
- Include-paths warning: `Paths can contain personal information. Include them only when you
  trust the person receiving these diagnostics.`
- Links: `Privacy policy`, `Source code`, `Open-source licences`, `Provider privacy policies`
- About: `SyncPak {version}`

# Roadmap

Everything described in this document is required for the first public release. Development
is divided into vertical milestones so risky platform behavior is tested early rather than
left until packaging.

## 1. Cross-platform Feasibility

- Establish minimal Slint applications on Linux, Android, and Windows in continuous builds.
- Prototype file/folder selection, protected credential storage, Android foreground-service
  execution, desktop notifications, and MSIX/Snap/Flatpak sandbox access.
- Prove basic authenticated list/upload/download/delete operations against test accounts for
  R2, B2, and S3.
- Record provider and platform limitations that affect capability abstractions or UI copy.

Exit criterion: every target can securely save a test credential and transfer a file while
using its intended packaging/security model.

## 2. Domain and Persistence Foundation

- Define versioned, serializable provider metadata and connection configurations.
- Implement atomic JSON writes, schema migration, validation, stable IDs, and secure-secret
  references.
- Separate small modules for configuration, credential storage, filesystem access, provider
  capabilities, comparison/planning, execution, queueing, and UI models. Keep each focused
  on one responsibility and normally below 200 lines of code.
- Add redaction-safe structured errors and diagnostics.

Exit criterion: configurations survive restart and migration without exposing credentials.

## 3. Provider Capability Layer

- Implement capabilities for listing, reading, writing, multipart upload, metadata,
  deletion, and bucket listing; do not require every provider to implement every capability.
- Add Cloudflare R2 and AWS S3 through the shared S3-compatible transport while retaining
  provider-specific configuration and explanations.
- Add Backblaze B2 behind the same capability contracts.
- Build conformance tests using isolated buckets/prefixes and failure injection.

Exit criterion: all providers pass the same supported-capability behavior suite.

## 4. Inventory, Comparison, and Planning

- Implement case-sensitive relative-path inventory, UTF-8/case collision preflight, hidden
  files, empty directories, symlinks, and best-effort metadata discovery.
- Implement type, size, and normalized modification-time comparison.
- Produce immutable operation plans for read-only, mirror, and archive directions without
  changing either endpoint.
- Test large trees, Unicode, timestamp precision differences, missing timestamps, overlapping paths,
  and platform-specific filesystem behavior.

Exit criterion: golden tests demonstrate every source/destination state produces the documented plan.

## 5. Safe Transfer Executor

- Implement temporary local downloads, provider-confirmed uploads, multipart cleanup, bounded retries,
  cancellation, progress events, and copy-before-delete ordering.
- Deliver read-only upload, download, and additive both-ways operation first.
- Add stateless mirror with plan invalidation and destructive confirmation requirements.
- Add archive creation in both directions, portable naming, temporary-file cleanup, and
  keep-last-N pruning.

Exit criterion: interruption and injected-failure tests never replace a good local file with
an incomplete file, never delete before required copies finish, and never prune archives
before a new archive is stored successfully.

## 6. Queue and Background Execution

- Implement the single-worker in-memory queue, immutable history snapshots, cancellation,
  deletion interactions, snackbars, and progress/result models.
- Connect Android execution to a foreground service and persistent notification.
- Clean stale temporary data safely at startup.

Exit criterion: queued operations execute in order and cancellation, app backgrounding, and
configuration deletion behave as documented on all targets.

## 7. Complete User Interface

- Build welcome, connection, provider, direction, preflight, progress, activity, privacy,
  diagnostics, and confirmation screens using the source copy in this document.
- Implement responsive desktop/touch layouts, connection cards, native pickers, bucket-list
  fallback, virtualized file lists, and all empty/loading/error states.
- Complete keyboard, screen-reader, contrast, scalable-text, focus, and reduced-motion testing.

Exit criterion: all documented flows are usable without a mouse and on the smallest supported
Android layout, and destructive actions are never ambiguous.

## 8. Privacy, Ads, and Release Integration

- Publish the privacy policy and provider disclosures; audit logs, errors, clipboard actions,
  and diagnostics for secret/path leakage.
- Add Android consent handling and AdMob only after privacy-sensitive screens are excluded.
- Produce signed Snap, Flatpak, Android, and MSIX packages using release CI.
- Test clean install, upgrade, uninstall, credential persistence/removal, sandbox permissions,
  background operation, and provider access for every package.

Exit criterion: release candidates pass privacy review and platform packaging tests.

## 9. Release Hardening

- Run end-to-end matrices across all modes, directions, providers, and target platforms.
- Exercise cancellation, process termination, offline transitions, throttling, invalid
  credentials, permission loss, full disks, archive collisions, and mass mirror deletion.
- Complete dependency/licence review, security review, accessibility audit, translation-ready
  string extraction, and user documentation.
- Release only when there are no known paths to silent data loss and every warning/error
  gives the user a concrete next action.
