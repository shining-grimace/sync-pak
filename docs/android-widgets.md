# Android widget device checklist

Build with `./android/gradlew --project-dir android :app:assembleDebug`. The APK is
`android/app/build/outputs/apk/debug/app-debug.apk` and contains ARM64 and x86_64.

1. Install the APK on a test device and configure verified upload-only, download-only,
   and both-ways Add Only connections. Use a disposable folder/bucket for transfers.
   Configure a separate preview-enabled connection and one unverified connection.
2. Add **SyncPak** and **SyncPak Compact** from the launcher's widget picker. Select
   different connections. Confirm the preview-enabled connection is omitted, the
   picker explains the omission, and the unverified entry offers **Verify in app**.
3. Check the standard widget's icon/name/explanation and compact widget's icon without
   the app name or explanation. Resize both. Check long names, large text, empty
   lists, and screen-reader labels. An empty widget should offer **Open SyncPak**.
4. Close the app without force-stopping it. Tap Run on a widget. Confirm no app screen
   opens, the foreground notification appears, and both widgets disable Run. Compact
   should show only the running connection instead of its selected connection.
5. Open the app during a widget run. Confirm Activity contains the same operation,
   another run cannot start, and the active connection/provider cannot be modified.
   Navigate around and return home; the operation should continue.
6. Cancel from the notification and separately from Activity. Confirm the operation
   stops at its next safe boundary, results are retained, the ongoing notification
   disappears and widgets become usable again. Repeat during preparation.
7. Finish a run. Confirm the completion notification reports transfers and skipped
   files, and tapping it opens that exact saved result. Repeat with no changes,
   inaccessible credentials/folders, network failure and a failed transfer.
8. Start a run from the app and confirm both widgets reflect it. Rapidly tap Run on
   multiple widgets; only one operation may be admitted.
9. Change a selected connection to require preview, delete it, or unverify it. Check
   the widget refreshes and that any stale Run action cannot bypass the new policy.
10. Restart the Android process, then run from a widget before opening the app.
    Check Keystore credentials, TLS and document-tree access work without Activity
    initialization. Open the app afterwards and confirm it uses the same runtime.
11. Interrupt the process during a run. On next startup, confirm interrupted history
    is reported and the sync is not silently repeated. Test notification permission
    denied and granted; saved results must remain accessible in either case.
12. Select Light and Dark in the app's appearance settings, including selections
    opposite to the phone's theme. Both widgets should use the app's primary colour
    for their outlines, text and icons, with its app background as the fill. Select
    System and switch the phone's theme with the app open and closed. Check disabled
    Run buttons remain distinguishable and widget sizes remain unchanged.

Automated validation covers Rust admission/eligibility and application-context
registration, Android target compilation, Kotlin and RemoteViews resource compilation,
and APK packaging. Real launcher interaction and Android service lifetime require
this device test; build-time checks cannot establish those behaviors.
