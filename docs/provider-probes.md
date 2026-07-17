# Provider feasibility probes

The test-only `provider_operations` example verifies one isolated prefix using
S3-compatible operations: list, upload, download and content verification, and
delete. It prints no endpoint, bucket, object key, credential, or object content.

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
cargo run --example provider_operations --features provider-probes
```

R2 uses its account S3 endpoint and `auto` region. B2 uses its regional S3
endpoint and a manually created Backblaze application key: its key ID is
`SYNCPAK_PROBE_ACCESS_KEY_ID` and its application key is
`SYNCPAK_PROBE_SECRET_ACCESS_KEY`. The key name is not used. This ordinary key
pair works with B2's S3-compatible API; do not use the automatically created
master application key. AWS S3 uses its normal region; omit
`SYNCPAK_PROBE_ENDPOINT` for the default AWS endpoint.

The probe attempts deletion after every successful upload, including when the
download or verification fails. A cleanup failure is reported as such, so the
operator can remove the single generated test object from the isolated prefix.
