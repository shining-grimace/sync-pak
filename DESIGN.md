
# App Overview

This app, with stylised name "SyncPak", is a GUI tool for synchronising local directories with cloud providers.

Vision: Create a free-to-use, no-nonsense, privacy-focused tool that's confiigured once and then syncs directories between places effortlessly.

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
- (To write: suggestion for Windows)

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

To write: how credentials should be managed securely using platform features

To write: summary of what should be included in the UI to alleviate privacy concerns

To write: visual design for displaying configured sync connections to enhance usability instead of hurting it (show local paths, remote providers and paths, which connections are archives, etc.)

# Provider Configs

A section of the app is dedicated to listing, adding, editing, verifying, and deleting the saved provider configs.

Provider config should:
- Focus on important, required fields and hide advances fields which would rarely get used

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

The aim of read-only mode is to download new files. Changed files show a warning in the UI and get skipped over, and missing files are not at all noteworthy.

If there are filesystem errors when running an operation, the precise error should be shown in the UI.

To write: what distinguishes changed and unchanged files in a robust but user-friendly way.

Archive mode does a zip of a directory locally and stores it wherever needed: on the local machine (if running remote to local) or in the cloud (if running from local to remote). The stored file will be named with a UTC timestamp (formatted as YYYYMMDD-HHMMSS appended with Z) followed by a space and the user-friendly connection name. If the file already exists, it should be a failure to try to create it again. After an archive is uploaded to cloud, the local copy should be deleted. To write: handling of non-ASCII characters

Archive zips are only deleted as per settings after one has successfully saved. The keep-last-N policy applies to both local and remote archives.

To initiate a sync, the user chooses the direction: upload, download, or both ways. Both ways is only available for read-only mode connections.

The sync operation occurs while a modal UI is shown; this has a loading indicator as needed, and will list files that are done or in progress, leaving the UI shown at the end so the user can browse the list of affected files before dismissing the modal.

In mirror mode, if there are any deletes or overwrites that would happen, these need to be listed in the modal UI and require confirmation before the sync starts.

Hidden files, empty directories, symlinks, and filesystem permissions are to be included in all sync operations. These will be best-effort, and warnings will be given to the user if they cannot be handled properly due to platform-specific or provider-specific behaviours. Symlinks will be copied as the links themselves (not the underlying file).

File names are case-sensitive (differences on case are considered separate files).

Failures to resolve things due to casing issues on the host platform or issues around UTF-8 support should be considered fatal issues that block the sync from being started.

Sync connections cannot run concurrently, but there should be a queue of them which can be viewed as a list in the UI somewhere. While running, the app should keep this list of sync operations, with status showing as queued, failed, completed, or in progress. Since it's only kept in memory, it doesn't persist across app launches. In case a connection or provider is deleted after a sync operation, this list of memory shouldn't reference any other data states apart from an ID needed for knowing where to apply status updates.

In progress connections can be cancelled from the queue list or from a notification-like popup UI at the bottom of the app UI, functioning kind of like a snackbar to show what's in progress.

On Android, if the app is sent to the background while a sync is running, the sync needs to keep running to completion and show a persistent notification during that time.

To write: good practices for failure and recovery behaviours

# UI Designs

To write: list of screens (list views, create/edit forms, modals, etc.) and their purposes

# Copy

To write: all the text needed in the entire app, in sections (Provider Configs list, New Provider form, etc.) and subsections (descriptions, error messages, etc.)

# Roadmap

To write: logical sequence for building all of the described features (everything will be in the first release)
