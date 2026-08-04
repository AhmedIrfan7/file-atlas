//! Typed queries against the atlas index.
//!
//! Every write is a single small function that takes `&mut Connection` or a
//! `Transaction<'_>` and returns a typed result. Callers compose these into
//! larger operations. No dynamic SQL. No string concatenation.

use rusqlite::{params, Connection, OptionalExtension, Row, Transaction};

use crate::models::{FileRow, VolumeRow};

/// Upsert a single file row. Preserves `first_seen`, always bumps `last_seen`,
/// and clears `removed_at` if the file has reappeared.
pub fn upsert_file(tx: &Transaction<'_>, row: &FileRow) -> rusqlite::Result<i64> {
    tx.execute(
        r"
        INSERT INTO files (
            path, parent, name, extension, size_bytes,
            created_at, modified_at, accessed_at,
            hash_blake3, hash_size, category,
            is_dir, is_hidden, is_symlink,
            volume_id, first_seen, last_seen, removed_at
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5,
            ?6, ?7, ?8,
            ?9, ?10, ?11,
            ?12, ?13, ?14,
            ?15, ?16, ?17, NULL
        )
        ON CONFLICT(path) DO UPDATE SET
            parent      = excluded.parent,
            name        = excluded.name,
            extension   = excluded.extension,
            size_bytes  = excluded.size_bytes,
            created_at  = COALESCE(excluded.created_at, files.created_at),
            modified_at = excluded.modified_at,
            accessed_at = excluded.accessed_at,
            is_dir      = excluded.is_dir,
            is_hidden   = excluded.is_hidden,
            is_symlink  = excluded.is_symlink,
            volume_id   = excluded.volume_id,
            last_seen   = excluded.last_seen,
            removed_at  = NULL
        ",
        params![
            row.path,
            row.parent,
            row.name,
            row.extension,
            row.size_bytes,
            row.created_at,
            row.modified_at,
            row.accessed_at,
            row.hash_blake3,
            row.hash_size,
            row.category,
            row.is_dir,
            row.is_hidden,
            row.is_symlink,
            row.volume_id,
            row.first_seen,
            row.last_seen,
        ],
    )?;
    tx.query_row(
        "SELECT id FROM files WHERE path = ?1",
        params![row.path],
        |r| r.get::<_, i64>(0),
    )
}

/// Insert-if-missing, always-bump `last_seen` for a volume.
pub fn upsert_volume(tx: &Transaction<'_>, row: &VolumeRow) -> rusqlite::Result<()> {
    tx.execute(
        r"
        INSERT INTO volumes (id, label, fs_type, mount, total_bytes, first_seen, last_seen)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        ON CONFLICT(id) DO UPDATE SET
            label       = excluded.label,
            fs_type     = excluded.fs_type,
            mount       = excluded.mount,
            total_bytes = excluded.total_bytes,
            last_seen   = excluded.last_seen
        ",
        params![
            row.id,
            row.label,
            row.fs_type,
            row.mount,
            row.total_bytes,
            row.first_seen,
            row.last_seen,
        ],
    )?;
    Ok(())
}

/// Read all known volumes.
pub fn list_volumes(conn: &Connection) -> rusqlite::Result<Vec<VolumeRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, label, fs_type, mount, total_bytes, first_seen, last_seen FROM volumes ORDER BY id",
    )?;
    let rows = stmt.query_map([], row_to_volume)?;
    rows.collect()
}

/// Mark every `files` row under `root_prefix` that was NOT touched in the given
/// scan as `removed_at = now`. Used to close out an incremental rescan.
pub fn mark_removed_since(
    tx: &Transaction<'_>,
    root_prefix: &str,
    scan_ts: i64,
    now: i64,
) -> rusqlite::Result<usize> {
    let escaped = escape_like(root_prefix);
    let forward_pattern = format!("{escaped}/%");
    let back_pattern = format!("{escaped}\\%");
    tx.execute(
        r"
        UPDATE files
        SET removed_at = ?3
        WHERE last_seen < ?2
          AND removed_at IS NULL
          AND (path = ?1 OR path LIKE ?4 ESCAPE '!' OR path LIKE ?5 ESCAPE '!')
        ",
        params![root_prefix, scan_ts, now, forward_pattern, back_pattern],
    )
}

/// Escape `%`, `_`, and the escape character itself (`!`) so a root prefix
/// containing any of them (e.g. a real folder named "50%_off", or a Windows
/// username like "John_Doe") is matched literally in the `LIKE` patterns
/// above rather than as a wildcard. Without this, rescanning
/// "C:\Users\John_Doe" could incorrectly mark files under the unrelated
/// "C:\Users\JohnXDoe" as removed, since `_` matches any single character.
/// Mirrors `atlas_core::storage_map`'s and `atlas_search::planner`'s
/// identically-named helpers, kept as its own copy since this crate cannot
/// depend on either without a circular dependency.
fn escape_like(s: &str) -> String {
    s.replace('!', "!!").replace('%', "!%").replace('_', "!_")
}

/// Count of rows currently visible (not removed).
pub fn count_live_files(conn: &Connection) -> rusqlite::Result<i64> {
    conn.query_row(
        "SELECT COUNT(*) FROM files WHERE removed_at IS NULL",
        [],
        |r| r.get(0),
    )
}

/// Fetch one row by path. Returns `None` if absent.
pub fn get_file_by_path(conn: &Connection, path: &str) -> rusqlite::Result<Option<FileRow>> {
    conn.query_row(
        r"SELECT
            path, parent, name, extension, size_bytes,
            created_at, modified_at, accessed_at,
            hash_blake3, hash_size, category,
            is_dir, is_hidden, is_symlink,
            volume_id, first_seen, last_seen, removed_at
           FROM files WHERE path = ?1",
        params![path],
        row_to_file,
    )
    .optional()
}

fn row_to_file(r: &Row<'_>) -> rusqlite::Result<FileRow> {
    Ok(FileRow {
        path: r.get(0)?,
        parent: r.get(1)?,
        name: r.get(2)?,
        extension: r.get(3)?,
        size_bytes: r.get(4)?,
        created_at: r.get(5)?,
        modified_at: r.get(6)?,
        accessed_at: r.get(7)?,
        hash_blake3: r.get(8)?,
        hash_size: r.get(9)?,
        category: r.get(10)?,
        is_dir: r.get::<_, i64>(11)? != 0,
        is_hidden: r.get::<_, i64>(12)? != 0,
        is_symlink: r.get::<_, i64>(13)? != 0,
        volume_id: r.get(14)?,
        first_seen: r.get(15)?,
        last_seen: r.get(16)?,
        removed_at: r.get(17)?,
    })
}

fn row_to_volume(r: &Row<'_>) -> rusqlite::Result<VolumeRow> {
    Ok(VolumeRow {
        id: r.get(0)?,
        label: r.get(1)?,
        fs_type: r.get(2)?,
        mount: r.get(3)?,
        total_bytes: r.get(4)?,
        first_seen: r.get(5)?,
        last_seen: r.get(6)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{connection::open_in_memory, migrations::apply};
    use pretty_assertions::assert_eq;

    fn make_conn() -> Connection {
        let mut conn = open_in_memory().expect("open");
        apply(&mut conn).expect("migrate");
        conn
    }

    fn sample_volume() -> VolumeRow {
        VolumeRow {
            id: "vol:test".into(),
            label: Some("Test".into()),
            fs_type: Some("NTFS".into()),
            mount: "C:\\".into(),
            total_bytes: Some(1_000_000),
            first_seen: 10,
            last_seen: 10,
        }
    }

    fn sample_file(path: &str, size: i64, seen: i64) -> FileRow {
        FileRow {
            path: path.into(),
            parent: "C:\\Users\\test".into(),
            name: "note.txt".into(),
            extension: Some("txt".into()),
            size_bytes: size,
            created_at: Some(seen),
            modified_at: Some(seen),
            accessed_at: Some(seen),
            hash_blake3: None,
            hash_size: None,
            category: Some("Document".into()),
            is_dir: false,
            is_hidden: false,
            is_symlink: false,
            volume_id: "vol:test".into(),
            first_seen: seen,
            last_seen: seen,
            removed_at: None,
        }
    }

    #[test]
    fn upsert_and_get_file_roundtrips() {
        let mut conn = make_conn();
        {
            let tx = conn.transaction().unwrap();
            upsert_volume(&tx, &sample_volume()).unwrap();
            let id = upsert_file(&tx, &sample_file("C:\\a\\note.txt", 42, 100)).unwrap();
            assert!(id > 0);
            tx.commit().unwrap();
        }
        let got = get_file_by_path(&conn, "C:\\a\\note.txt").unwrap().unwrap();
        assert_eq!(got.size_bytes, 42);
    }

    #[test]
    fn upsert_file_is_idempotent_and_bumps_last_seen() {
        let mut conn = make_conn();
        {
            let tx = conn.transaction().unwrap();
            upsert_volume(&tx, &sample_volume()).unwrap();
            let a = upsert_file(&tx, &sample_file("C:\\a\\note.txt", 10, 100)).unwrap();
            let b = upsert_file(&tx, &sample_file("C:\\a\\note.txt", 20, 200)).unwrap();
            assert_eq!(a, b, "same path should reuse the same id");
            tx.commit().unwrap();
        }
        let got = get_file_by_path(&conn, "C:\\a\\note.txt").unwrap().unwrap();
        assert_eq!(got.size_bytes, 20);
        assert_eq!(got.last_seen, 200);
    }

    #[test]
    fn list_volumes_returns_upserted_row() {
        let mut conn = make_conn();
        {
            let tx = conn.transaction().unwrap();
            upsert_volume(&tx, &sample_volume()).unwrap();
            tx.commit().unwrap();
        }
        let vols = list_volumes(&conn).unwrap();
        assert_eq!(vols.len(), 1);
        assert_eq!(vols[0].id, "vol:test");
    }

    #[test]
    fn mark_removed_since_does_not_treat_underscore_as_wildcard() {
        // "John_Doe" is a real, common Windows username. Unescaped, the `_`
        // in a LIKE pattern matches any single character, so rescanning
        // "C:\Users\John_Doe" could incorrectly mark files under the
        // unrelated "C:\Users\JohnXDoe" as removed too.
        let mut conn = make_conn();
        let tx = conn.transaction().unwrap();
        upsert_volume(&tx, &sample_volume()).unwrap();
        upsert_file(&tx, &sample_file("C:\\Users\\John_Doe\\note.txt", 1, 100)).unwrap();
        upsert_file(&tx, &sample_file("C:\\Users\\JohnXDoe\\note.txt", 1, 100)).unwrap();
        let touched = mark_removed_since(&tx, "C:\\Users\\John_Doe", 400, 1_000).unwrap();
        assert_eq!(
            touched, 1,
            "only the real John_Doe file should be swept, not the unrelated JohnXDoe one"
        );
        tx.commit().unwrap();

        let real = get_file_by_path(&conn, "C:\\Users\\John_Doe\\note.txt")
            .unwrap()
            .unwrap();
        let decoy = get_file_by_path(&conn, "C:\\Users\\JohnXDoe\\note.txt")
            .unwrap()
            .unwrap();
        assert_eq!(real.removed_at, Some(1_000));
        assert_eq!(
            decoy.removed_at, None,
            "an unrelated folder must not be swept just for sharing prefix characters"
        );
    }

    #[test]
    fn mark_removed_since_flags_untouched_files() {
        let mut conn = make_conn();
        {
            let tx = conn.transaction().unwrap();
            upsert_volume(&tx, &sample_volume()).unwrap();
            upsert_file(&tx, &sample_file("C:\\r\\stale.txt", 1, 100)).unwrap();
            upsert_file(&tx, &sample_file("C:\\r\\fresh.txt", 1, 500)).unwrap();
            let touched = mark_removed_since(&tx, "C:\\r", 400, 1_000).unwrap();
            assert_eq!(touched, 1);
            tx.commit().unwrap();
        }
        let stale = get_file_by_path(&conn, "C:\\r\\stale.txt")
            .unwrap()
            .unwrap();
        let fresh = get_file_by_path(&conn, "C:\\r\\fresh.txt")
            .unwrap()
            .unwrap();
        assert_eq!(stale.removed_at, Some(1_000));
        assert_eq!(fresh.removed_at, None);
        assert_eq!(count_live_files(&conn).unwrap(), 1);
    }
}
