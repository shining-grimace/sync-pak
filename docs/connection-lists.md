# Portable connection lists

Settings contains About, then Connection Lists, Appearance, Local Data, and Privacy.
Connection Lists has one row of actions: **Import JSON**, **Export JSON**, and
**Edit Root**. It does not show saved list headings or connection details.
Welcome's privacy button reads **Privacy**.

## Local Root

**Edit Root** opens a single **Local Root** setting and a folder picker. Its initial
value is the platform home directory. The selected root is stored once, as the
`local_root` field in settings. There are no per-connection or per-provider roots,
and this screen does not list connections.

Local Root is the base for portable local paths. A connection configured at
`/home/alex/Documents/Projects`, with Local Root `/home/alex`, exports the relative
path `Documents/Projects`. Importing on a device whose Local Root is
`C:\Users\Sam` resolves it beneath that directory.

Changing Local Root preserves existing configured connection destinations and
verification. Paths beneath the new root are inferred relative to it. Other paths
remain usable in the app but are incompatible with portable export. The export
review displays `(root)/...` for compatible paths and `Incompatible: ...` for
others; incompatible entries cannot be selected.

On Android, use the folder picker to choose a document tree and grant access.
Imported descendants resolve through DocumentsContract. Separate document-tree
URIs that cannot safely be related to the selected root are incompatible; the app
does not infer filesystem relationships from opaque document IDs.

## Remote identity

Remote identity is the pair **provider type + bucket name**. Remote paths are
relative to that bucket. Exported JSON contains this pair and the path; it contains
no provider UUID, bucket UUID, account ID, endpoint, region, or credentials.

Import first looks for providers of the matching type already associated with the
bucket, through an existing connection or the provider's configured default bucket.
If none is associated, it considers configured providers of that type. A unique
candidate is selected automatically. Ambiguous matches require an explicit provider
choice during import. Provider display names and device-specific credentials do not
participate in portable identity.

## Import and export

Choose **Import JSON**, select a file, and review its connections. Local paths use
this device's Local Root; remote destinations use the inferred provider and bucket.
Resolve ambiguous provider choices and choose which connections to import.
Duplicate connection IDs offer **Skip**, **Replace**, and **Import as copy**; Skip
is the default. Copies get new connection IDs. New and replaced connections are
unverified and require verification before syncing.

Choose **Export JSON**, select compatible connections in the review, then **Save
JSON**. Only the selected connections are exported. Credentials, local absolute
paths, verification, activity, and appearance are excluded.

Imports and root edits are staged until confirmation. Cancel or Escape leaves
settings unchanged. A concurrent settings change causes the review to reject a
stale save. Import changes are validated and saved atomically; importing never
starts a transfer. Cancelling a file dialog leaves the review available.

## File schemas

See [example connection list](examples/connection-list.json). The transport uses
`format: "syncpak-connection-list"`, `version: 1`, and a `connections` array.
Documents are limited to 4 MiB. Each entry has a stable connection ID, relative
`local_path`, `remote` containing `provider_kind`, `bucket`, and `path`, plus sync
mode, allowed directions, and optional archive retention.

Paths use `/` separators. Empty paths refer to the root or bucket. Imported paths
cannot be absolute or contain drive prefixes, backslashes, empty components, `.`
or `..`. Resolution never depends on the working directory or JSON file location.

The settings schema remains **version 1**, with one `local_root`, device-local
providers, and connections. Local connection paths beneath Local Root are stored
relatively and resolved on load. Incompatible destinations are stored absolutely
so changing Local Root does not relocate them. No named-root tables, saved-list
objects, migrations, or compatibility readers are included. Previous developer
settings must be removed manually if they use the superseded schema.

## Device checks

- Confirm the three actions occupy a single row at narrow and wide window sizes.
- On a fresh configuration, open Edit Root and check the home-directory default.
  Pick another root, save, restart, and confirm it persists without moving folders.
- Export compatible connections, verify `(root)/...` paths, and confirm incompatible
  entries are disabled. Import on another device using a different Local Root and
  differently named provider with the same provider type and bucket name.
- Exercise ambiguous providers, Skip/Replace/Copy, invalid JSON, cancellation,
  denied document access, and failed writes.
- On Android, select a tree and exercise descendant folders containing spaces,
  Unicode, `%`, and `#` through verification and transfers.
