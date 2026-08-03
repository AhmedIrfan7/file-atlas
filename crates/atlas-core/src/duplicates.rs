//! Groups hashed files by content and suggests which copy to keep.
//!
//! Grouping is a pure read against already-hashed rows (see `hasher`); it
//! never touches the filesystem. Groups are ordered by wasted space
//! (`size_bytes * (count - 1)`) so the biggest space-savers surface first.

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::analytics::FileSummary;

/// One member of a duplicate group, with whether it is the suggested keeper.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DuplicateMember {
    pub file: FileSummary,
    pub suggested_keep: bool,
}

/// A set of files sharing the same content hash.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DuplicateGroup {
    pub hash: String,
    pub size_bytes: i64,
    pub wasted_bytes: i64,
    pub keep_reason: String,
    pub members: Vec<DuplicateMember>,
}

/// The `limit` duplicate groups with the most wasted space, each carrying a
/// suggested file to keep (the most recently modified copy; ties broken by
/// path so the choice is stable).
pub fn find_duplicate_groups(
    conn: &Connection,
    limit: u32,
) -> rusqlite::Result<Vec<DuplicateGroup>> {
    let mut group_stmt = conn.prepare(
        "SELECT hash_blake3, size_bytes, COUNT(*) as cnt
         FROM files
         WHERE removed_at IS NULL AND is_dir = 0 AND hash_blake3 IS NOT NULL
         GROUP BY hash_blake3
         HAVING cnt > 1
         ORDER BY size_bytes * (cnt - 1) DESC
         LIMIT ?1",
    )?;
    let hashes: Vec<(String, i64)> = group_stmt
        .query_map(rusqlite::params![limit], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let mut member_stmt = conn.prepare(
        "SELECT path, name, size_bytes, modified_at, category
         FROM files
         WHERE removed_at IS NULL AND is_dir = 0 AND hash_blake3 = ?1
         ORDER BY path",
    )?;

    let mut groups = Vec::with_capacity(hashes.len());
    for (hash, size_bytes) in hashes {
        let mut files: Vec<FileSummary> = member_stmt
            .query_map(rusqlite::params![hash], |r| {
                Ok(FileSummary {
                    path: r.get(0)?,
                    name: r.get(1)?,
                    size_bytes: r.get(2)?,
                    modified_at: r.get(3)?,
                    category: r.get(4)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        // Newest modified_at is the suggested keeper; None sorts as oldest.
        // Ties (including all-None) fall back to path for a stable choice.
        files.sort_by(|a, b| a.path.cmp(&b.path));
        let keep_index = files
            .iter()
            .enumerate()
            .max_by_key(|(_, f)| (f.modified_at, std::cmp::Reverse(&f.path)))
            .map_or(0, |(i, _)| i);

        let members = files
            .into_iter()
            .enumerate()
            .map(|(i, file)| DuplicateMember {
                file,
                suggested_keep: i == keep_index,
            })
            .collect::<Vec<_>>();

        let wasted_bytes = size_bytes * (i64::try_from(members.len()).unwrap_or(i64::MAX) - 1);

        groups.push(DuplicateGroup {
            hash,
            size_bytes,
            wasted_bytes,
            keep_reason: "Keeping the most recently modified copy".to_string(),
            members,
        });
    }

    Ok(groups)
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_db::queries::{upsert_file, upsert_volume};
    use atlas_db::{apply_migrations, open_in_memory, FileRow, VolumeRow};

    fn make_conn() -> Connection {
        let mut c = open_in_memory().expect("open");
        apply_migrations(&mut c).expect("migrate");
        c
    }

    #[allow(clippy::too_many_arguments)]
    fn seed(conn: &Connection, path: &str, size: i64, modified: Option<i64>, hash: Option<&str>) {
        let tx = conn.unchecked_transaction().unwrap();
        upsert_volume(
            &tx,
            &VolumeRow {
                id: "vol:test".into(),
                label: None,
                fs_type: None,
                mount: "C:\\".into(),
                total_bytes: None,
                first_seen: 0,
                last_seen: 0,
            },
        )
        .unwrap();
        upsert_file(
            &tx,
            &FileRow {
                path: path.to_string(),
                parent: "C:\\r".into(),
                name: path.rsplit(['\\', '/']).next().unwrap_or(path).to_string(),
                extension: None,
                size_bytes: size,
                created_at: modified,
                modified_at: modified,
                accessed_at: modified,
                hash_blake3: hash.map(str::to_string),
                hash_size: hash.map(|_| size),
                category: Some("Other".into()),
                is_dir: false,
                is_hidden: false,
                is_symlink: false,
                volume_id: "vol:test".into(),
                first_seen: 0,
                last_seen: 0,
                removed_at: None,
            },
        )
        .unwrap();
        tx.commit().unwrap();
    }

    #[test]
    fn groups_files_sharing_a_hash() {
        let conn = make_conn();
        seed(&conn, "C:\\r\\a.bin", 100, Some(1), Some("h1"));
        seed(&conn, "C:\\r\\b.bin", 100, Some(2), Some("h1"));
        seed(&conn, "C:\\r\\c.bin", 50, Some(1), Some("h2"));

        let groups = find_duplicate_groups(&conn, 10).unwrap();
        assert_eq!(groups.len(), 1, "singleton hash h2 should not form a group");
        assert_eq!(groups[0].hash, "h1");
        assert_eq!(groups[0].members.len(), 2);
    }

    #[test]
    fn suggests_newest_as_keep() {
        let conn = make_conn();
        seed(&conn, "C:\\r\\old.bin", 100, Some(100), Some("h1"));
        seed(&conn, "C:\\r\\new.bin", 100, Some(999), Some("h1"));

        let groups = find_duplicate_groups(&conn, 10).unwrap();
        let keeper = groups[0].members.iter().find(|m| m.suggested_keep).unwrap();
        assert_eq!(keeper.file.name, "new.bin");
    }

    #[test]
    fn orders_groups_by_wasted_space_descending() {
        let conn = make_conn();
        // Group A: 2 copies of 1000 bytes = 1000 wasted.
        seed(&conn, "C:\\r\\a1.bin", 1000, Some(1), Some("ha"));
        seed(&conn, "C:\\r\\a2.bin", 1000, Some(2), Some("ha"));
        // Group B: 3 copies of 10 bytes = 20 wasted.
        seed(&conn, "C:\\r\\b1.bin", 10, Some(1), Some("hb"));
        seed(&conn, "C:\\r\\b2.bin", 10, Some(2), Some("hb"));
        seed(&conn, "C:\\r\\b3.bin", 10, Some(3), Some("hb"));

        let groups = find_duplicate_groups(&conn, 10).unwrap();
        assert_eq!(groups[0].hash, "ha");
        assert_eq!(groups[0].wasted_bytes, 1000);
        assert_eq!(groups[1].hash, "hb");
        assert_eq!(groups[1].wasted_bytes, 20);
    }

    #[test]
    fn unhashed_files_are_excluded() {
        let conn = make_conn();
        seed(&conn, "C:\\r\\a.bin", 100, Some(1), None);
        seed(&conn, "C:\\r\\b.bin", 100, Some(2), None);
        let groups = find_duplicate_groups(&conn, 10).unwrap();
        assert!(groups.is_empty());
    }
}
