//! Executes a `PlannedQuery` against a live connection and maps rows into
//! `SearchHit`. The only layer here that touches SQLite; `parser` and
//! `planner` stay pure and DB-free.

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::parser::SearchQuery;
use crate::planner::plan;

/// One row returned from a search.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchHit {
    pub path: String,
    pub name: String,
    pub size_bytes: i64,
    pub modified_at: Option<i64>,
    pub category: Option<String>,
    pub is_dir: bool,
}

/// Run `query` against `conn`. `now_unix` anchors age filters (`age>1y`
/// means "modified more than a year before `now_unix`"); `limit` bounds the
/// number of rows returned.
pub fn search(
    conn: &Connection,
    query: &SearchQuery,
    now_unix: i64,
    limit: u32,
) -> rusqlite::Result<Vec<SearchHit>> {
    let planned = plan(query, now_unix, limit);
    let mut stmt = conn.prepare(&planned.sql)?;
    let params = rusqlite::params_from_iter(planned.params.iter());
    let rows = stmt.query_map(params, |r| {
        Ok(SearchHit {
            path: r.get(0)?,
            name: r.get(1)?,
            size_bytes: r.get(2)?,
            modified_at: r.get(3)?,
            category: r.get(4)?,
            is_dir: r.get::<_, i64>(5)? != 0,
        })
    })?;
    rows.collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;
    use atlas_db::queries::{upsert_file, upsert_volume};
    use atlas_db::{apply_migrations, open_in_memory, FileRow, VolumeRow};

    fn make_conn() -> Connection {
        let mut c = open_in_memory().expect("open");
        apply_migrations(&mut c).expect("migrate");
        c
    }

    fn seed(conn: &mut Connection, files: &[(&str, &str, i64, Option<i64>, &str)]) {
        let tx = conn.transaction().unwrap();
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
        for (path, name, size, modified, category) in files {
            let extension = name.rsplit('.').next().filter(|e| *e != *name);
            upsert_file(
                &tx,
                &FileRow {
                    path: (*path).to_string(),
                    parent: "C:\\r".into(),
                    name: (*name).to_string(),
                    extension: extension.map(str::to_string),
                    size_bytes: *size,
                    created_at: *modified,
                    modified_at: *modified,
                    accessed_at: *modified,
                    hash_blake3: None,
                    hash_size: None,
                    category: Some((*category).to_string()),
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
        }
        tx.commit().unwrap();
    }

    #[test]
    fn free_text_matches_by_name_prefix() {
        let mut conn = make_conn();
        seed(
            &mut conn,
            &[
                (
                    "C:\\r\\resume_final.pdf",
                    "resume_final.pdf",
                    100,
                    Some(1_000),
                    "Document",
                ),
                ("C:\\r\\notes.txt", "notes.txt", 50, Some(1_000), "Document"),
            ],
        );
        let q = parse("resume").unwrap();
        let hits = search(&conn, &q, 2_000, 50).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].name, "resume_final.pdf");
    }

    #[test]
    fn extension_filter_narrows_results() {
        let mut conn = make_conn();
        seed(
            &mut conn,
            &[
                ("C:\\r\\a.pdf", "a.pdf", 100, Some(1_000), "Document"),
                ("C:\\r\\b.txt", "b.txt", 100, Some(1_000), "Document"),
            ],
        );
        let q = parse("type:pdf").unwrap();
        let hits = search(&conn, &q, 2_000, 50).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].name, "a.pdf");
    }

    #[test]
    fn size_and_age_filters_combine() {
        let mut conn = make_conn();
        let now = 10_000_000i64;
        let one_year = 365 * 86_400;
        seed(
            &mut conn,
            &[
                (
                    "C:\\r\\big_old.bin",
                    "big_old.bin",
                    10_000_000,
                    Some(now - one_year - 1),
                    "Other",
                ),
                (
                    "C:\\r\\big_new.bin",
                    "big_new.bin",
                    10_000_000,
                    Some(now - 10),
                    "Other",
                ),
                (
                    "C:\\r\\small_old.bin",
                    "small_old.bin",
                    10,
                    Some(now - one_year - 1),
                    "Other",
                ),
            ],
        );
        let q = parse("size>1mb age>1y").unwrap();
        let hits = search(&conn, &q, now, 50).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].name, "big_old.bin");
    }

    #[test]
    fn no_query_returns_most_recently_modified_first() {
        let mut conn = make_conn();
        seed(
            &mut conn,
            &[
                ("C:\\r\\old.txt", "old.txt", 10, Some(100), "Document"),
                ("C:\\r\\new.txt", "new.txt", 10, Some(999), "Document"),
            ],
        );
        let q = parse("").unwrap();
        let hits = search(&conn, &q, 2_000, 50).unwrap();
        assert_eq!(hits[0].name, "new.txt");
    }

    #[test]
    fn removed_files_are_excluded() {
        let mut conn = make_conn();
        seed(
            &mut conn,
            &[("C:\\r\\gone.txt", "gone.txt", 10, Some(100), "Document")],
        );
        conn.execute(
            "UPDATE files SET removed_at = 999 WHERE name = 'gone.txt'",
            [],
        )
        .unwrap();
        let q = parse("gone").unwrap();
        let hits = search(&conn, &q, 2_000, 50).unwrap();
        assert!(hits.is_empty());
    }

    #[test]
    fn folder_filter_does_not_treat_underscore_as_wildcard() {
        // "John_Doe" is a real, common Windows username. Unescaped, the `_`
        // in a LIKE pattern matches any single character, so `in:John_Doe`
        // would also match the unrelated folder "JohnXDoe".
        let mut conn = make_conn();
        let tx = conn.transaction().unwrap();
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
        for (path, parent) in [
            ("C:\\Users\\John_Doe\\resume.pdf", "C:\\Users\\John_Doe"),
            ("C:\\Users\\JohnXDoe\\resume.pdf", "C:\\Users\\JohnXDoe"),
        ] {
            upsert_file(
                &tx,
                &FileRow {
                    path: path.to_string(),
                    parent: parent.to_string(),
                    name: "resume.pdf".to_string(),
                    extension: Some("pdf".to_string()),
                    size_bytes: 10,
                    created_at: Some(1),
                    modified_at: Some(1),
                    accessed_at: Some(1),
                    hash_blake3: None,
                    hash_size: None,
                    category: Some("Document".to_string()),
                    is_dir: false,
                    is_hidden: false,
                    is_symlink: false,
                    volume_id: "vol:test".to_string(),
                    first_seen: 0,
                    last_seen: 0,
                    removed_at: None,
                },
            )
            .unwrap();
        }
        tx.commit().unwrap();

        let q = parse("in:John_Doe").unwrap();
        let hits = search(&conn, &q, 2_000, 50).unwrap();
        assert_eq!(
            hits.len(),
            1,
            "underscore in the folder filter must match literally, not as a wildcard"
        );
        assert_eq!(hits[0].path, "C:\\Users\\John_Doe\\resume.pdf");
    }
}
