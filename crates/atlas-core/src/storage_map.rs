//! On-demand storage aggregation for the treemap view.
//!
//! There is no maintained "folder size" column: `files` rows only carry a
//! leaf file's own `size_bytes` (directories are always stored as 0, see
//! `FileRecord::from_metadata`). A folder's displayed size is computed on
//! request by summing every live file whose path falls under that folder,
//! using an indexed `LIKE 'prefix\%' ESCAPE '!'` scan rather than a
//! maintained rollup table. See ADR 0008 for why: at the scope of one
//! drill-down level (a handful to a few dozen immediate children) this is
//! fast and needs no scan-time bookkeeping; a maintained rollup is the
//! documented next step if it ever proves too slow.

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

/// One rectangle in the treemap: either a real folder (drillable) or the
/// synthetic "files directly in this folder" bucket (a leaf).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageNode {
    pub path: String,
    pub name: String,
    pub is_dir: bool,
    pub size_bytes: i64,
}

/// The current view: either the top-level list of scanned roots (`scope
/// path` is `None`) or one folder's immediate children plus its loose files.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageMapResponse {
    pub scope_path: Option<String>,
    pub total_bytes: i64,
    pub nodes: Vec<StorageNode>,
}

/// Optional narrowing applied to every size computation in this view.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageMapFilter {
    pub category: Option<String>,
    pub since_unix: Option<i64>,
}

/// Build the treemap view for `scope_path` (the top-level scan-root list if
/// `None`), narrowed by `filter`.
#[allow(clippy::option_if_let_else)] // two named branches read clearer than map_or_else here
pub fn get_storage_map(
    conn: &Connection,
    scope_path: Option<&str>,
    filter: &StorageMapFilter,
) -> rusqlite::Result<StorageMapResponse> {
    match scope_path {
        None => root_level(conn, filter),
        Some(path) => drill_down(conn, path, filter),
    }
}

fn root_level(
    conn: &Connection,
    filter: &StorageMapFilter,
) -> rusqlite::Result<StorageMapResponse> {
    let mut stmt =
        conn.prepare("SELECT DISTINCT root FROM scans WHERE status = 'completed' ORDER BY root")?;
    let roots: Vec<String> = stmt
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let mut nodes = Vec::with_capacity(roots.len());
    let mut total_bytes = 0i64;
    for root in roots {
        let size = subtree_size(conn, &root, filter)?;
        total_bytes += size;
        nodes.push(StorageNode {
            name: display_name(&root),
            path: root,
            is_dir: true,
            size_bytes: size,
        });
    }
    nodes.sort_by_key(|n| std::cmp::Reverse(n.size_bytes));

    Ok(StorageMapResponse {
        scope_path: None,
        total_bytes,
        nodes,
    })
}

fn drill_down(
    conn: &Connection,
    scope_path: &str,
    filter: &StorageMapFilter,
) -> rusqlite::Result<StorageMapResponse> {
    let mut stmt = conn.prepare(
        "SELECT path, name FROM files
         WHERE parent = ?1 AND is_dir = 1 AND removed_at IS NULL
         ORDER BY path",
    )?;
    let subfolders: Vec<(String, String)> = stmt
        .query_map(rusqlite::params![scope_path], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let mut nodes = Vec::with_capacity(subfolders.len() + 1);
    let mut total_bytes = 0i64;
    for (path, name) in subfolders {
        let size = subtree_size(conn, &path, filter)?;
        total_bytes += size;
        nodes.push(StorageNode {
            path,
            name,
            is_dir: true,
            size_bytes: size,
        });
    }

    let loose = direct_files_size(conn, scope_path, filter)?;
    if loose > 0 {
        total_bytes += loose;
        nodes.push(StorageNode {
            path: scope_path.to_string(),
            name: "(files in this folder)".to_string(),
            is_dir: false,
            size_bytes: loose,
        });
    }

    nodes.sort_by_key(|n| std::cmp::Reverse(n.size_bytes));

    Ok(StorageMapResponse {
        scope_path: Some(scope_path.to_string()),
        total_bytes,
        nodes,
    })
}

/// Total live, filtered file size under `root` (the folder itself and every
/// descendant). Uses an escaped `LIKE` prefix scan against the indexed
/// `path` column rather than walking `parent` links recursively.
fn subtree_size(conn: &Connection, root: &str, filter: &StorageMapFilter) -> rusqlite::Result<i64> {
    let pattern = format!("{}\\%", escape_like(root));
    let category_clause = category_clause(filter, 3);
    let since_clause = since_clause(filter, 4);
    let sql = format!(
        "SELECT COALESCE(SUM(size_bytes), 0) FROM files
         WHERE is_dir = 0 AND removed_at IS NULL
           AND (path = ?1 OR path LIKE ?2 ESCAPE '!')
           {category_clause} {since_clause}"
    );
    conn.query_row(
        &sql,
        rusqlite::params![root, pattern, filter.category, filter.since_unix],
        |r| r.get(0),
    )
}

/// Total size of files that live directly inside `folder`, not in any
/// subfolder.
fn direct_files_size(
    conn: &Connection,
    folder: &str,
    filter: &StorageMapFilter,
) -> rusqlite::Result<i64> {
    let category_clause = category_clause(filter, 2);
    let since_clause = since_clause(filter, 3);
    let sql = format!(
        "SELECT COALESCE(SUM(size_bytes), 0) FROM files
         WHERE is_dir = 0 AND removed_at IS NULL AND parent = ?1
           {category_clause} {since_clause}"
    );
    conn.query_row(
        &sql,
        rusqlite::params![folder, filter.category, filter.since_unix],
        |r| r.get(0),
    )
}

/// SQL fragment narrowing by category, bound to placeholder `?{idx}`.
/// Direct equality when a category is set (lets SQLite use
/// `idx_files_category`); an `IS NULL OR` fallback when it is not, so the
/// same bind value (`NULL`) always sits at that position regardless of
/// whether this clause is "active".
fn category_clause(filter: &StorageMapFilter, idx: u32) -> String {
    if filter.category.is_some() {
        format!("AND category = ?{idx}")
    } else {
        format!("AND (?{idx} IS NULL OR category = ?{idx})")
    }
}

fn since_clause(filter: &StorageMapFilter, idx: u32) -> String {
    if filter.since_unix.is_some() {
        format!("AND modified_at >= ?{idx}")
    } else {
        format!("AND (?{idx} IS NULL OR modified_at >= ?{idx})")
    }
}

/// Escape `%`, `_`, and the escape character itself (`!`) so a path
/// containing any of them is matched literally rather than as a wildcard.
/// Backslash cannot be the escape character here since Windows paths use it
/// as the separator.
fn escape_like(s: &str) -> String {
    s.replace('!', "!!").replace('%', "!%").replace('_', "!_")
}

fn display_name(path: &str) -> String {
    path.rsplit(['\\', '/'])
        .find(|s| !s.is_empty())
        .unwrap_or(path)
        .to_string()
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

    fn seed_scan(conn: &Connection, root: &str) {
        conn.execute(
            "INSERT INTO scans (root, started_at, finished_at, status) VALUES (?1, 0, 1, 'completed')",
            rusqlite::params![root],
        )
        .unwrap();
    }

    #[allow(clippy::too_many_arguments)]
    fn seed_file(
        conn: &Connection,
        path: &str,
        parent: &str,
        name: &str,
        size: i64,
        modified: Option<i64>,
        category: &str,
        is_dir: bool,
    ) {
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
                parent: parent.to_string(),
                name: name.to_string(),
                extension: None,
                size_bytes: size,
                created_at: modified,
                modified_at: modified,
                accessed_at: modified,
                hash_blake3: None,
                hash_size: None,
                category: Some(category.to_string()),
                is_dir,
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
    fn root_level_lists_distinct_completed_scan_roots() {
        let conn = make_conn();
        seed_scan(&conn, "C:\\Users\\me\\Desktop");
        seed_scan(&conn, "C:\\Users\\me\\Desktop"); // rescan, should not duplicate
        seed_scan(&conn, "C:\\Users\\me\\Downloads");
        seed_file(
            &conn,
            "C:\\Users\\me\\Desktop\\a.txt",
            "C:\\Users\\me\\Desktop",
            "a.txt",
            100,
            Some(1),
            "Document",
            false,
        );

        let resp = get_storage_map(&conn, None, &StorageMapFilter::default()).unwrap();
        assert_eq!(resp.nodes.len(), 2);
        assert!(resp.nodes.iter().all(|n| n.is_dir));
    }

    #[test]
    fn drill_down_separates_subfolders_from_loose_files() {
        let conn = make_conn();
        seed_file(
            &conn,
            "C:\\r\\sub",
            "C:\\r",
            "sub",
            0,
            Some(1),
            "Folder",
            true,
        );
        seed_file(
            &conn,
            "C:\\r\\sub\\deep.txt",
            "C:\\r\\sub",
            "deep.txt",
            500,
            Some(1),
            "Document",
            false,
        );
        seed_file(
            &conn,
            "C:\\r\\loose.txt",
            "C:\\r",
            "loose.txt",
            200,
            Some(1),
            "Document",
            false,
        );

        let resp = get_storage_map(&conn, Some("C:\\r"), &StorageMapFilter::default()).unwrap();
        assert_eq!(resp.nodes.len(), 2);
        let sub = resp.nodes.iter().find(|n| n.name == "sub").unwrap();
        assert!(sub.is_dir);
        assert_eq!(sub.size_bytes, 500);
        let loose = resp.nodes.iter().find(|n| !n.is_dir).unwrap();
        assert_eq!(loose.size_bytes, 200);
        assert_eq!(resp.total_bytes, 700);
    }

    #[test]
    fn category_filter_narrows_subtree_totals() {
        let conn = make_conn();
        seed_file(
            &conn,
            "C:\\r\\a.jpg",
            "C:\\r",
            "a.jpg",
            300,
            Some(1),
            "Image",
            false,
        );
        seed_file(
            &conn,
            "C:\\r\\b.txt",
            "C:\\r",
            "b.txt",
            700,
            Some(1),
            "Document",
            false,
        );

        let filter = StorageMapFilter {
            category: Some("Image".to_string()),
            since_unix: None,
        };
        let resp = get_storage_map(&conn, Some("C:\\r"), &filter).unwrap();
        assert_eq!(resp.total_bytes, 300);
    }

    #[test]
    fn since_filter_excludes_older_files() {
        let conn = make_conn();
        seed_file(
            &conn,
            "C:\\r\\old.txt",
            "C:\\r",
            "old.txt",
            1000,
            Some(100),
            "Document",
            false,
        );
        seed_file(
            &conn,
            "C:\\r\\new.txt",
            "C:\\r",
            "new.txt",
            500,
            Some(900),
            "Document",
            false,
        );

        let filter = StorageMapFilter {
            category: None,
            since_unix: Some(500),
        };
        let resp = get_storage_map(&conn, Some("C:\\r"), &filter).unwrap();
        assert_eq!(resp.total_bytes, 500);
    }

    #[test]
    fn folder_names_with_percent_and_underscore_are_matched_literally() {
        let conn = make_conn();
        // A sibling whose name is a substring-with-wildcard-meaning of the
        // scoped folder must not be swept in by an unescaped LIKE pattern.
        seed_file(
            &conn,
            "C:\\r\\100%_done",
            "C:\\r",
            "100%_done",
            0,
            Some(1),
            "Folder",
            true,
        );
        seed_file(
            &conn,
            "C:\\r\\100%_done\\inside.txt",
            "C:\\r\\100%_done",
            "inside.txt",
            42,
            Some(1),
            "Document",
            false,
        );
        seed_file(
            &conn,
            "C:\\r\\100X_done_extra.txt",
            "C:\\r",
            "100X_done_extra.txt",
            999,
            Some(1),
            "Document",
            false,
        );

        let total = subtree_size(&conn, "C:\\r\\100%_done", &StorageMapFilter::default()).unwrap();
        assert_eq!(total, 42, "must not match the unrelated sibling file");
    }
}
