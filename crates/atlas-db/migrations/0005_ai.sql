-- Local AI layer (M9): per-file name embeddings for semantic search, plus a
-- small key-value settings table for AI configuration (cloud opt-in, model
-- choice, endpoint). Embeddings are stored as raw little-endian f32 bytes in
-- a BLOB rather than JSON: fixed-size, no parse overhead, and cosine
-- similarity only ever needs the raw floats back out.

CREATE TABLE file_embeddings (
    path       TEXT PRIMARY KEY,
    model      TEXT NOT NULL,
    dims       INTEGER NOT NULL,
    vector     BLOB NOT NULL,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (path) REFERENCES files(path) ON DELETE CASCADE
);

CREATE INDEX idx_file_embeddings_model ON file_embeddings(model);

CREATE TABLE ai_settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
