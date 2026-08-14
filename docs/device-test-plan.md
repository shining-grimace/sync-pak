# Pre-release device and release-candidate test plan

Run this plan for every release candidate after a clean install and again after an in-place
upgrade from the preceding candidate. This is the acceptance gate for behaviour, edge cases,
accessibility, sandboxing, and platform compatibility that are intentionally deferred from
feature implementation.

Use isolated, least-privilege test buckets or prefixes. Never place credentials, real user
files, object listings, or provider response bodies in test records. Record the app build,
package type, operating-system version, device/model, provider, result, and any defect for
each run.

## Coverage matrix

Run the core flow for every provider on every supported target, not merely one paired row.

| Providers | Target packages |
| --- | --- |
| Cloudflare R2, Backblaze B2, and AWS S3 | Linux Flatpak and Snap on supported desktop runtimes; Android 11+ ARM64 APK/AAB on a physical device; Windows 10+ installed MSIX |

Use the smallest supported Android layout and maximum font/display size. On desktop, run the
keyboard checks without a mouse. Run Android accessibility checks with TalkBack enabled.

## Package, upgrade, and permissions

- **Clean install:** Install the final candidate package, launch it, and confirm no
  developer probes or feasibility controls are exposed.
- **Upgrade and uninstall:** Upgrade an existing install and confirm configuration and
  protected credentials behave as promised. Uninstall, reinstall, and confirm application
  data and credentials are either retained or removed according to the documented package
  behaviour.
- **Sandbox and folder access:** Select a local folder through the native picker, relaunch,
  and complete a transfer. Revoke or remove that grant, then confirm the app reports a useful
  recovery action without broad filesystem access. Check Flatpak and Snap interface/portal
  connections, Android Storage Access Framework permission, and installed-MSIX access.
- **Protected storage:** Save credentials, relaunch, and verify a provider without exposing
  secret values in UI, diagnostics, clipboard, logs, or configuration. Exercise unavailable
  protected storage where the package permits it and confirm there is no plaintext fallback.
- **Notifications:** Check desktop notification delivery in installed Linux and Windows
  packages. On Android, grant and deny notification permission and confirm the foreground
  service remains understandable in both cases.

## Core operation matrix

For each provider/package combination, use a small tree containing nested directories, a
hidden file, an empty directory, Unicode names, and at least one file that differs between
local and remote. Retain the test tree until all scenarios below have completed.

- **Launch and persistence:** Create and verify a provider and connection, relaunch, and
  confirm both remain usable without displaying credentials.
- **Add-only:** Complete upload, download, and both-ways runs. Confirm new files transfer,
  pre-existing changed files are not silently overwritten, and the result/activity history
  accurately describes the outcome.
- **Mirror:** Review a plan containing copies and deletions. Confirm it cannot start until
  destructive acknowledgement is given, copies occur before deletions, and the result lists
  affected items.
- **Archive:** With `keep last = 1`, run archive twice in each supported direction. Confirm a
  usable archive is produced, only the earlier SyncPak-recorded archive is pruned, and an
  untracked file is never removed.
- **Queue and deletion:** Queue at least two operations and confirm submission order. Cancel a
  queued operation and an active operation. Delete a connection and then a provider with
  associated work, confirming queued work is removed and active work is cancelled first.
- **Background (Android):** Start a long operation, leave the app, confirm foreground
  notification and progress, reopen the app, then cancel from both notification and Activity.
  Confirm the service stops when the queue is idle.

## Failure, interruption, and edge cases

- **Preflight invalidation:** Change a reviewed local or remote file before starting; the app
  must require a refreshed review and make no destination change.
- **Connectivity and credentials:** Disconnect the network, use invalid credentials, and
  restore service. Errors must identify a practical next action; a retry must not duplicate
  completed work.
- **Transfer safety:** Interrupt a download and upload during transfer, including multipart
  upload. A good local destination must remain intact, incomplete uploads must be cleaned up
  or retained only as documented, and no mirror delete may occur before required copies finish.
- **Storage and permissions:** Simulate insufficient local space, a read-only or revoked local
  folder, and provider permission loss. The app must fail safely, retain recoverable data, and
  avoid silent data loss.
- **Path and inventory edges:** Exercise large trees, long paths, Unicode names, case-only
  conflicts where supported, missing timestamps, and implicit remote directories. Fatal
  conflicts must stop the run before changes.
- **Termination and recovery:** Force-close during an active operation, relaunch, and confirm
  stale SyncPak temporary data is cleaned safely without removing user files.

## Accessibility, layout, privacy, and release checks

- **Access:** Complete the primary flow with TalkBack on Android and keyboard navigation on
  Linux and Windows. Verify labels, live status, focus order, focus restoration, destructive
  confirmations, Escape/Back, contrast, and non-colour status cues.
- **Layout:** At maximum display/font size, confirm no clipped text, hidden actions, unusable
  scrolling, or ambiguous destructive controls. Check narrow Android and resized desktop
  windows, light/dark/system appearance, and reduced-motion behaviour.
- **Privacy and diagnostics:** Copy diagnostics with and without paths. Confirm credentials
  never appear and paths appear only after explicit opt-in. Verify privacy policy, licences,
  source, and provider-policy links resolve correctly in release packages.
- **Release review:** Confirm final version metadata, signing, update channel, dependency and
  licence review, and known-issues list. Do not ship with a known path to silent data loss or
  an error/warning that lacks a concrete next action.
