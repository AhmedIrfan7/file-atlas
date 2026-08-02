-- Initial schema for the File Atlas index.
--
-- Rows are per-entry (file OR directory). The `is_dir` flag distinguishes them.
-- `first_seen` is set on the very first upsert; `last_seen` is bumped every scan.
-- `removed_at` is set when a scan no longer finds the entry (soft-delete for undo).

CREATE TABLE volumes (
    id         TEXT PRIMARY KEY,        -- opaque, stable across drive-letter changes
    label      TEXT,
    fs_type    TEXT,
    mount      TEXT NOT NULL,
    total_bytes INTEGER,
    first_seen INTEGER NOT NULL,
    last_seen  INTEGER NOT NULL
);

CREATE TABLE files (
    id            INTEGER PRIMARY KEY,
    path          TEXT NOT NULL UNIQUE,
    parent        TEXT NOT NULL,
    name          TEXT NOT NULL,
    extension     TEXT,
    size_bytes    INTEGER NOT NULL DEFAULT 0,
    created_at    INTEGER,
    modified_at   INTEGER,
    accessed_at   INTEGER,
    hash_blake3   TEXT,
    hash_size     INTEGER,
    category      TEXT,
    is_dir        INTEGER NOT NULL DEFAULT 0,
    is_hidden     INTEGER NOT NULL DEFAULT 0,
    is_symlink    INTEGER NOT NULL DEFAULT 0,
    volume_id     TEXT NOT NULL,
    first_seen    INTEGER NOT NULL,
    last_seen     INTEGER NOT NULL,
    removed_at    INTEGER,
    FOREIGN KEY (volume_id) REFERENCES volumes(id) ON DELETE RESTRICT
);

CREATE INDEX idx_files_parent   ON files(parent);
CREATE INDEX idx_files_hash     ON files(hash_blake3);
CREATE INDEX idx_files_size     ON files(size_bytes DESC);
CREATE INDEX idx_files_modified ON files(modified_at);
CREATE INDEX idx_files_category ON files(category);
CREATE INDEX idx_files_removed  ON files(removed_at);
CREATE INDEX idx_files_volume   ON files(volume_id);

CREATE TABLE scans (
    id           INTEGER PRIMARY KEY,
    root         TEXT NOT NULL,
    started_at   INTEGER NOT NULL,
    finished_at  INTEGER,
    files_seen   INTEGER NOT NULL DEFAULT 0,
    bytes_seen   INTEGER NOT NULL DEFAULT 0,
    status       TEXT NOT NULL DEFAULT 'running'
        CHECK (status IN ('running','completed','cancelled','failed'))
);

CREATE INDEX idx_scans_root ON scans(root);

CREATE TABLE actions_log (
    id          INTEGER PRIMARY KEY,
    ts          INTEGER NOT NULL,
    op          TEXT NOT NULL
        CHECK (op IN ('trash','restore','move','rename','permanent_delete','protect','unprotect')),
    path_from   TEXT,
    path_to     TEXT,
    metadata    TEXT,        -- JSON blob
    reversible  INTEGER NOT NULL DEFAULT 1,
    undo_ref    TEXT
);

CREATE INDEX idx_actions_ts ON actions_log(ts DESC);

CREATE TABLE protected_paths (
    path   TEXT PRIMARY KEY,
    reason TEXT NOT NULL,
    added_at INTEGER NOT NULL
);
