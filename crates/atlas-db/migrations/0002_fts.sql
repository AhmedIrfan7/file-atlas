-- Full-text search over file names and paths.
--
-- Uses SQLite FTS5 with an external content link to the `files` table so we
-- do not duplicate storage. Triggers keep the FTS index in sync with row
-- inserts, updates, and deletes.

CREATE VIRTUAL TABLE files_fts USING fts5(
    name,
    path,
    content='files',
    content_rowid='id',
    tokenize='unicode61 remove_diacritics 2'
);

CREATE TRIGGER files_fts_insert AFTER INSERT ON files BEGIN
    INSERT INTO files_fts(rowid, name, path) VALUES (new.id, new.name, new.path);
END;

CREATE TRIGGER files_fts_delete AFTER DELETE ON files BEGIN
    INSERT INTO files_fts(files_fts, rowid, name, path) VALUES ('delete', old.id, old.name, old.path);
END;

CREATE TRIGGER files_fts_update AFTER UPDATE ON files BEGIN
    INSERT INTO files_fts(files_fts, rowid, name, path) VALUES ('delete', old.id, old.name, old.path);
    INSERT INTO files_fts(rowid, name, path) VALUES (new.id, new.name, new.path);
END;
