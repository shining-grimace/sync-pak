# Provider feasibility probes

The test-only `provider_operations` example runs SyncPak's shared conformance suite
against one isolated prefix: list, upload, download, metadata, content verification,
delete, multipart completion, and multipart abort. It transfers approximately 5 MiB for
the multipart check and prints no endpoint, bucket, object key, credential, or object content.

Use a dedicated test bucket or a restricted, disposable prefix. The credential
must have only `ListBucket`, `GetObject`, `PutObject`, and `DeleteObject` access
to that scope. Never use production credentials.

```text
export SYNCPAK_PROBE_PROVIDER=cloudflare-r2 # aws-s3, backblaze-b2, or cloudflare-r2
export SYNCPAK_PROBE_ACCESS_KEY_ID=...
export SYNCPAK_PROBE_SECRET_ACCESS_KEY=...
export SYNCPAK_PROBE_BUCKET=...
export SYNCPAK_PROBE_PREFIX=syncpak-feasibility
export SYNCPAK_PROBE_ENDPOINT=https://... # required for R2 and B2
export SYNCPAK_PROBE_REGION=auto           # R2; use the B2 region or AWS region otherwise
# Optional: require account-level bucket enumeration and check this bucket is present.
export SYNCPAK_PROBE_CHECK_BUCKET_LISTING=false
cargo run --example provider_operations --features provider-probes
```

R2 uses its account S3 endpoint and `auto` region. B2 uses its regional S3
endpoint and a manually created Backblaze application key: its key ID is
`SYNCPAK_PROBE_ACCESS_KEY_ID` and its application key is
`SYNCPAK_PROBE_SECRET_ACCESS_KEY`. The key name is not used. This ordinary key
pair works with B2's S3-compatible API; do not use the automatically created
master application key. AWS S3 uses its normal region; omit
`SYNCPAK_PROBE_ENDPOINT` for the default AWS endpoint.

The probe attempts deletion after every successful upload, including when an
intermediate operation fails. A cleanup failure takes precedence, so the operator can
remove the single generated test object from the isolated prefix. Bucket enumeration is
opt-in because a least-privilege credential can validly be allowed to list one bucket's
objects without being allowed to list every bucket in the account.

Accepted provider-run evidence is recorded in `docs/provider-conformance.md`.
