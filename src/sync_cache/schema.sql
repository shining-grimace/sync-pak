CREATE TABLE IF NOT EXISTS remote_observations (
    namespace TEXT NOT NULL,
    bucket TEXT NOT NULL,
    object_key TEXT NOT NULL,
    byte_size TEXT NOT NULL,
    modified INTEGER,
    etag TEXT,
    source_modified INTEGER,
    last_seen INTEGER NOT NULL,
    PRIMARY KEY (namespace, bucket, object_key)
);

CREATE TABLE IF NOT EXISTS baselines (
    namespace TEXT NOT NULL,
    path TEXT NOT NULL,
    local_kind INTEGER NOT NULL,
    local_size TEXT NOT NULL,
    local_modified INTEGER,
    remote_size TEXT NOT NULL,
    remote_modified INTEGER,
    remote_etag TEXT,
    effective_source INTEGER,
    last_seen INTEGER NOT NULL,
    PRIMARY KEY (namespace, path)
);

PRAGMA user_version = 1;
