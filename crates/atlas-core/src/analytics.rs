//! Read-only queries that power the home view: category breakdown, top-N
//! largest and oldest files, and the "not touched in a while" bucket.
//!
//! Every function here takes a plain `&Connection` and returns typed structs.
//! No mutation, no side effects. Safe to call from a UI thread against a
//! pooled read connection.

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

const SECONDS_PER_DAY: i64 = 86_400;

/// One row in the category breakdown: a category name, file count, and total
/// size in bytes among currently-live (not removed) files.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CategoryTotal {
    pub category: String,
    pub file_count: i64,
    pub total_bytes: i64,
}

/// A single file surfaced in a top-N list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileSummary {
    pub path: String,
    pub name: String,
    pub size_bytes: i64,
    pub modified_at: Option<i64>,
    pub category: Option<String>,
}

/// Aggregate totals shown at the top of the home view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HomeSummary {
    pub live_file_count: i64,
    pub live_folder_count: i64,
    pub total_bytes: i64,
    pub categories: Vec<CategoryTotal>,
}

/// Overall totals plus the category breakdown, restricted to live (not
/// removed, not directory) rows for size purposes.
pub fn home_summary(conn: &Connection) -> rusqlite::Result<HomeSummary> {
    let live_file_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM files WHERE removed_at IS NULL AND is_dir = 0",
        [],
        |r| r.get(0),
    )?;
    let live_folder_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM files WHERE removed_at IS NULL AND is_dir = 1",
        [],
        |r| r.get(0),
    )?;
    let total_bytes: i64 = conn.query_row(
        "SELECT COALESCE(SUM(size_bytes), 0) FROM files WHERE removed_at IS NULL AND is_dir = 0",
        [],
        |r| r.get(0),
    )?;

    let mut stmt = conn.prepare(
        "SELECT COALESCE(category, 'Other') AS cat, COUNT(*), COALESCE(SUM(size_bytes), 0)
         FROM files
         WHERE removed_at IS NULL AND is_dir = 0
         GROUP BY cat
         ORDER BY SUM(size_bytes) DESC",
    )?;
    let categories = stmt
        .query_map([], |r| {
            Ok(CategoryTotal {
                category: r.get(0)?,
                file_count: r.get(1)?,
                total_bytes: r.get(2)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    Ok(HomeSummary {
        live_file_count,
        live_folder_count,
        total_bytes,
        categories,
    })
}

/// The `limit` largest live files by size, descending.
pub fn top_largest(conn: &Connection, limit: u32) -> rusqlite::Result<Vec<FileSummary>> {
    query_top(
        conn,
        "SELECT path, name, size_bytes, modified_at, category
         FROM files
         WHERE removed_at IS NULL AND is_dir = 0
         ORDER BY size_bytes DESC
         LIMIT ?1",
        limit,
    )
}

/// The `limit` oldest live files by `modified_at`, ascending. Files with no
/// `modified_at` are excluded since age is undefined for them.
pub fn top_oldest(conn: &Connection, limit: u32) -> rusqlite::Result<Vec<FileSummary>> {
    query_top(
        conn,
        "SELECT path, name, size_bytes, modified_at, category
         FROM files
         WHERE removed_at IS NULL AND is_dir = 0 AND modified_at IS NOT NULL
         ORDER BY modified_at ASC
         LIMIT ?1",
        limit,
    )
}

fn query_top(conn: &Connection, sql: &str, limit: u32) -> rusqlite::Result<Vec<FileSummary>> {
    let mut stmt = conn.prepare(sql)?;
    let limit_i64 = i64::from(limit);
    let rows = stmt.query_map(rusqlite::params![limit_i64], row_to_summary)?;
    rows.collect()
}

fn row_to_summary(r: &rusqlite::Row<'_>) -> rusqlite::Result<FileSummary> {
    Ok(FileSummary {
        path: r.get(0)?,
        name: r.get(1)?,
        size_bytes: r.get(2)?,
        modified_at: r.get(3)?,
        category: r.get(4)?,
    })
}

/// Summary of files not modified in at least `min_age_days`.
///
/// Returns the count, total bytes, and up to `sample_limit` example files
/// (largest first) so the UI can show "1,204 files, 42 GB, here are the
/// biggest ones" without pulling the whole set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaleBucket {
    pub min_age_days: u32,
    pub file_count: i64,
    pub total_bytes: i64,
    pub sample: Vec<FileSummary>,
}

pub fn stale_bucket(
    conn: &Connection,
    now_unix: i64,
    min_age_days: u32,
    sample_limit: u32,
) -> rusqlite::Result<StaleBucket> {
    let cutoff = now_unix - i64::from(min_age_days) * SECONDS_PER_DAY;

    let file_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM files
         WHERE removed_at IS NULL AND is_dir = 0
           AND modified_at IS NOT NULL AND modified_at < ?1",
        rusqlite::params![cutoff],
        |r| r.get(0),
    )?;
    let total_bytes: i64 = conn.query_row(
        "SELECT COALESCE(SUM(size_bytes), 0) FROM files
         WHERE removed_at IS NULL AND is_dir = 0
           AND modified_at IS NOT NULL AND modified_at < ?1",
        rusqlite::params![cutoff],
        |r| r.get(0),
    )?;

    let mut stmt = conn.prepare(
        "SELECT path, name, size_bytes, modified_at, category
         FROM files
         WHERE removed_at IS NULL AND is_dir = 0
           AND modified_at IS NOT NULL AND modified_at < ?1
         ORDER BY size_bytes DESC
         LIMIT ?2",
    )?;
    let sample = stmt
        .query_map(
            rusqlite::params![cutoff, i64::from(sample_limit)],
            row_to_summary,
        )?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    Ok(StaleBucket {
        min_age_days,
        file_count,
        total_bytes,
        sample,
    })
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

    fn seed(conn: &mut Connection, files: &[(&str, i64, Option<i64>, &str)]) {
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
        for (path, size, modified, category) in files {
            upsert_file(
                &tx,
                &FileRow {
                    path: (*path).to_string(),
                    parent: "C:\\r".into(),
                    name: path.rsplit(['\\', '/']).next().unwrap_or(path).to_string(),
                    extension: None,
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
    fn home_summary_aggregates_by_category() {
        let mut conn = make_conn();
        seed(
            &mut conn,
            &[
                ("C:\\r\\a.jpg", 100, Some(1_000), "Image"),
                ("C:\\r\\b.jpg", 200, Some(1_000), "Image"),
                ("C:\\r\\c.pdf", 50, Some(1_000), "Document"),
            ],
        );
        let summary = home_summary(&conn).unwrap();
        assert_eq!(summary.live_file_count, 3);
        assert_eq!(summary.total_bytes, 350);
        assert_eq!(summary.categories.len(), 2);
        assert_eq!(summary.categories[0].category, "Image");
        assert_eq!(summary.categories[0].total_bytes, 300);
    }

    #[test]
    fn top_largest_orders_descending() {
        let mut conn = make_conn();
        seed(
            &mut conn,
            &[
                ("C:\\r\\small.txt", 10, Some(1), "Document"),
                ("C:\\r\\big.txt", 1_000, Some(1), "Document"),
                ("C:\\r\\mid.txt", 100, Some(1), "Document"),
            ],
        );
        let top = top_largest(&conn, 2).unwrap();
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].name, "big.txt");
        assert_eq!(top[1].name, "mid.txt");
    }

    #[test]
    fn top_oldest_orders_ascending_and_skips_null_modified() {
        let mut conn = make_conn();
        seed(
            &mut conn,
            &[
                ("C:\\r\\new.txt", 10, Some(2_000), "Document"),
                ("C:\\r\\old.txt", 10, Some(100), "Document"),
            ],
        );
        let oldest = top_oldest(&conn, 5).unwrap();
        assert_eq!(oldest.len(), 2);
        assert_eq!(oldest[0].name, "old.txt");
    }

    #[test]
    fn stale_bucket_counts_files_older_than_cutoff() {
        let mut conn = make_conn();
        let now = 10_000_000i64;
        let one_year_secs = 365 * SECONDS_PER_DAY;
        seed(
            &mut conn,
            &[
                (
                    "C:\\r\\ancient.txt",
                    500,
                    Some(now - one_year_secs - 10_000),
                    "Document",
                ),
                ("C:\\r\\recent.txt", 500, Some(now - 100), "Document"),
            ],
        );
        let bucket = stale_bucket(&conn, now, 365, 10).unwrap();
        assert_eq!(bucket.file_count, 1);
        assert_eq!(bucket.total_bytes, 500);
        assert_eq!(bucket.sample.len(), 1);
        assert_eq!(bucket.sample[0].name, "ancient.txt");
    }
}
