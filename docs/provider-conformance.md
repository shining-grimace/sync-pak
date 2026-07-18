# Provider capability conformance

This document records evidence for roadmap milestone 3. A provider passes only after the
test-only `provider_operations` example runs successfully with least-privilege credentials
against its isolated test bucket or prefix.

## Accepted evidence

On 2026-07-19, the shared S3-compatible transport passed for Cloudflare R2, Backblaze B2, and
AWS S3. Each run used the provider's existing isolated test credentials and completed:

- object listing, upload, readback, metadata lookup, deletion, and post-delete verification;
- multipart start, a 5 MiB first-part upload, final-part upload, completion, readback, and
  deletion; and
- a separate multipart abort.

The common conformance tests also inject a failed object read and a failed multipart part upload.
They verify that a successfully-created temporary object is deleted and that a failed multipart
upload is aborted.

Bucket enumeration remains an optional capability check because least-privilege credentials may
list objects in one bucket without permission to enumerate every bucket in an account.

## Result

Roadmap milestone 3, Provider Capability Layer, satisfies its exit criterion: every supported
provider passed the same supported-capability behavior suite. The credentials, endpoints, bucket
names, prefixes, object keys, and provider response bodies are intentionally not recorded here.
