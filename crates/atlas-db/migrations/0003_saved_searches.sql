-- User-saved search queries. `name` is unique so saving under an existing
-- name replaces it rather than accumulating duplicates.

CREATE TABLE saved_searches (
    id         INTEGER PRIMARY KEY,
    name       TEXT NOT NULL,
    query_text TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE UNIQUE INDEX idx_saved_searches_name ON saved_searches(name);
