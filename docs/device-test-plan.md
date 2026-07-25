# Device test plan

Run this short smoke test on each release candidate. Use the smallest supported Android layout,
maximum font/display size, and a real provider test bucket.

## Coverage

Test every provider on every target platform:

| Providers | Platforms |
| --- | --- |
| Cloudflare R2 | Linux (supported desktop runtime) |
| Backblaze B2 | Android 11+ on ARM64 |
| AWS S3 | Windows 10+ |

Run the whole list for each provider/platform pair. The **Background** check applies to Android;
the **keyboard** part of **Access** applies to Linux and Windows.

- **Launch and persistence:** Create a provider and connection, relaunch, and confirm both remain usable; credentials must not be shown.
- **Setup:** Verify the provider, select a native local folder, and complete an add-only run in each direction.
- **Safety:** A mirror with changes cannot start before acknowledgement; connection/provider deletion stops or removes its work first.
- **Recovery:** Make a reviewed file change or disconnect the network; the app explains the failure and can be retried.
- **Background:** Start a long run, leave the app, confirm foreground notification/progress, then cancel from both notification and Activity.
- **Archive:** With `keep last = 1`, run twice; prune only the earlier SyncPak-recorded archive, never an untracked file.
- **Access:** With TalkBack and keyboard navigation, complete the main flow; labels, focus, confirmations, and Escape/Back are clear.
- **Layout:** No clipped text, hidden actions, or ambiguous destructive controls at the maximum display/font size.
