//! Life timeline (M7): a chronological view of file creation, plus
//! auto-detected bursts of activity.
//!
//! Two independent things live here:
//!
//! - `get_timeline` buckets every live file's `created_at` into day or month
//!   periods, giving the histogram the timeline UI renders as bars.
//! - `screenshot_bursts` / `project_bursts` find days where an unusual number
//!   of files appeared at once: a screen-shotting session (any folder, image
//!   category, screenshot-style filename) or a folder suddenly gaining many
//!   files on the same day (an extracted archive, a cloned project, a batch
//!   download). See ADR 0009 for why "receipt clusters" and "semester
//!   periods" from the original roadmap sketch are not here yet.
//!
//! Bucketing happens in SQL (`strftime(..., 'start of day' | 'start of
//! month')`) rather than in Rust, so grouping millions of rows never means
//! pulling every row's timestamp across the FFI boundary to bucket by hand.

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::analytics::FileSummary;

/// How `created_at` timestamps are bucketed.
///
/// Only the two granularities the UI actually needs: "This week" renders as
/// daily bars, "This year" as monthly bars. A generic week/quarter picker is
/// not built until a real need for one shows up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Granularity {
    Day,
    Month,
}

impl Granularity {
    const fn sqlite_modifier(self) -> &'static str {
        match self {
            Self::Day => "start of day",
            Self::Month => "start of month",
        }
    }
}

/// One bar in the timeline.
///
/// How many files were created in this period, and how much space they take
/// up. `period_start` is the bucket's start as a Unix timestamp; the caller
/// already knows the granularity it asked for, so formatting the label is a
/// frontend concern.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimelineBucket {
    pub period_start: i64,
    pub file_count: i64,
    pub total_bytes: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimelineResponse {
    pub granularity: Granularity,
    pub buckets: Vec<TimelineBucket>,
}

/// Bucket every live file's `created_at` by `granularity`.
///
/// Optionally restricted to `created_at >= since_unix`. Files with no
/// `created_at` (rare, but possible when the OS denies metadata access) are
/// excluded: a period with unknown timing cannot be placed on a timeline.
pub fn get_timeline(
    conn: &Connection,
    granularity: Granularity,
    since_unix: Option<i64>,
) -> rusqlite::Result<TimelineResponse> {
    let modifier = granularity.sqlite_modifier();
    let sql = format!(
        "SELECT CAST(strftime('%s', created_at, 'unixepoch', '{modifier}') AS INTEGER) AS bucket,
                COUNT(*), COALESCE(SUM(size_bytes), 0)
         FROM files
         WHERE is_dir = 0 AND removed_at IS NULL AND created_at IS NOT NULL
           AND (?1 IS NULL OR created_at >= ?1)
         GROUP BY bucket
         ORDER BY bucket ASC"
    );
    let mut stmt = conn.prepare(&sql)?;
    let buckets = stmt
        .query_map(rusqlite::params![since_unix], |r| {
            Ok(TimelineBucket {
                period_start: r.get(0)?,
                file_count: r.get(1)?,
                total_bytes: r.get(2)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    Ok(TimelineResponse {
        granularity,
        buckets,
    })
}

/// One detected burst.
///
/// `folder` is `None` for a cross-folder screenshot burst (the whole point
/// is that screenshots land wherever the OS puts them) and `Some(parent)`
/// for a project burst (the whole point is that it is one folder gaining
/// many files at once).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Burst {
    pub kind: String,
    pub folder: Option<String>,
    pub period_start: i64,
    pub file_count: i64,
    pub total_bytes: i64,
    pub sample: Vec<FileSummary>,
}

/// A day needs at least this many screenshots to be flagged as a burst.
///
/// Fixed, like the recommender's thresholds (see `atlas_recommender::engine`):
/// a tuning UI is speculative surface until someone actually needs one.
pub const SCREENSHOT_BURST_MIN_COUNT: u32 = 6;
/// A folder needs at least this many same-day file creations to be flagged
/// as a project burst.
pub const PROJECT_BURST_MIN_COUNT: u32 = 8;
/// How many example files to return per burst.
pub const BURST_SAMPLE_LIMIT: u32 = 12;

const SCREENSHOT_NAME_PATTERNS: &str = "
               LOWER(name) LIKE 'screenshot%'
               OR LOWER(name) LIKE 'screen shot%'
               OR LOWER(name) LIKE 'screen_shot%'";

/// Days where at least `min_count` screenshot-named images were created,
/// anywhere in the index.
///
/// Reuses the same filename heuristic as M5's `screenshot_pileups` rule, but
/// clusters by creation day instead of by folder: a pileup is "this folder
/// has a lot of screenshots ever", a burst is "you took a lot of screenshots
/// in one sitting".
pub fn screenshot_bursts(
    conn: &Connection,
    min_count: u32,
    sample_limit: u32,
) -> rusqlite::Result<Vec<Burst>> {
    let bucket_sql = format!(
        "SELECT CAST(strftime('%s', created_at, 'unixepoch', 'start of day') AS INTEGER) AS bucket,
                COUNT(*), COALESCE(SUM(size_bytes), 0)
         FROM files
         WHERE is_dir = 0 AND removed_at IS NULL AND created_at IS NOT NULL
           AND category = 'Image' AND ({SCREENSHOT_NAME_PATTERNS})
         GROUP BY bucket
         HAVING COUNT(*) >= ?1
         ORDER BY bucket DESC"
    );
    let mut bucket_stmt = conn.prepare(&bucket_sql)?;
    let buckets: Vec<(i64, i64, i64)> = bucket_stmt
        .query_map(rusqlite::params![min_count], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let sample_sql = format!(
        "SELECT path, name, size_bytes, modified_at, category
         FROM files
         WHERE is_dir = 0 AND removed_at IS NULL AND created_at IS NOT NULL
           AND category = 'Image' AND ({SCREENSHOT_NAME_PATTERNS})
           AND CAST(strftime('%s', created_at, 'unixepoch', 'start of day') AS INTEGER) = ?1
         ORDER BY created_at ASC
         LIMIT ?2"
    );
    let mut sample_stmt = conn.prepare(&sample_sql)?;

    let mut bursts = Vec::with_capacity(buckets.len());
    for (bucket, file_count, total_bytes) in buckets {
        let sample = sample_stmt
            .query_map(
                rusqlite::params![bucket, i64::from(sample_limit)],
                row_to_summary,
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        bursts.push(Burst {
            kind: "screenshot_burst".to_string(),
            folder: None,
            period_start: bucket,
            file_count,
            total_bytes,
            sample,
        });
    }
    Ok(bursts)
}

/// Folder-and-day pairs where at least `min_count` files were created on the
/// same day in the same parent folder.
///
/// The signature of extracting an archive, cloning a project, or a batch
/// download landing all at once.
pub fn project_bursts(
    conn: &Connection,
    min_count: u32,
    sample_limit: u32,
) -> rusqlite::Result<Vec<Burst>> {
    let mut bucket_stmt = conn.prepare(
        "SELECT parent,
                CAST(strftime('%s', created_at, 'unixepoch', 'start of day') AS INTEGER) AS bucket,
                COUNT(*), COALESCE(SUM(size_bytes), 0)
         FROM files
         WHERE is_dir = 0 AND removed_at IS NULL AND created_at IS NOT NULL
         GROUP BY parent, bucket
         HAVING COUNT(*) >= ?1
         ORDER BY COUNT(*) DESC",
    )?;
    let groups: Vec<(String, i64, i64, i64)> = bucket_stmt
        .query_map(rusqlite::params![min_count], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let mut sample_stmt = conn.prepare(
        "SELECT path, name, size_bytes, modified_at, category
         FROM files
         WHERE is_dir = 0 AND removed_at IS NULL AND created_at IS NOT NULL
           AND parent = ?1
           AND CAST(strftime('%s', created_at, 'unixepoch', 'start of day') AS INTEGER) = ?2
         ORDER BY created_at ASC
         LIMIT ?3",
    )?;

    let mut bursts = Vec::with_capacity(groups.len());
    for (parent, bucket, file_count, total_bytes) in groups {
        let sample = sample_stmt
            .query_map(
                rusqlite::params![parent, bucket, i64::from(sample_limit)],
                row_to_summary,
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        bursts.push(Burst {
            kind: "project_burst".to_string(),
            folder: Some(parent),
            period_start: bucket,
            file_count,
            total_bytes,
            sample,
        });
    }
    Ok(bursts)
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
    fn seed_file(
        conn: &Connection,
        path: &str,
        parent: &str,
        name: &str,
        size: i64,
        created: Option<i64>,
        category: &str,
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
                created_at: created,
                modified_at: created,
                accessed_at: created,
                hash_blake3: None,
                hash_size: None,
                category: Some(category.to_string()),
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

    // Two known instants, far enough apart to land in different days AND
    // different months, so day and month grouping can be told apart.
    const DAY1: i64 = 1_700_000_000; // 2023-11-14 22:13:20 UTC
    const DAY2: i64 = 1_700_100_000; // 2023-11-16 01:20:00 UTC (next day)
    const NEXT_MONTH: i64 = 1_702_600_000; // 2023-12-14 21:26:40 UTC

    #[test]
    fn day_granularity_separates_different_days() {
        let conn = make_conn();
        seed_file(
            &conn,
            "C:\\r\\a.txt",
            "C:\\r",
            "a.txt",
            100,
            Some(DAY1),
            "Document",
        );
        seed_file(
            &conn,
            "C:\\r\\b.txt",
            "C:\\r",
            "b.txt",
            200,
            Some(DAY1 + 60),
            "Document",
        );
        seed_file(
            &conn,
            "C:\\r\\c.txt",
            "C:\\r",
            "c.txt",
            300,
            Some(DAY2),
            "Document",
        );

        let resp = get_timeline(&conn, Granularity::Day, None).unwrap();
        assert_eq!(resp.buckets.len(), 2);
        assert_eq!(resp.buckets[0].file_count, 2);
        assert_eq!(resp.buckets[0].total_bytes, 300);
        assert_eq!(resp.buckets[1].file_count, 1);
    }

    #[test]
    fn month_granularity_merges_days_within_a_month() {
        let conn = make_conn();
        seed_file(
            &conn,
            "C:\\r\\a.txt",
            "C:\\r",
            "a.txt",
            100,
            Some(DAY1),
            "Document",
        );
        seed_file(
            &conn,
            "C:\\r\\b.txt",
            "C:\\r",
            "b.txt",
            200,
            Some(DAY2),
            "Document",
        );
        seed_file(
            &conn,
            "C:\\r\\c.txt",
            "C:\\r",
            "c.txt",
            300,
            Some(NEXT_MONTH),
            "Document",
        );

        let resp = get_timeline(&conn, Granularity::Month, None).unwrap();
        assert_eq!(resp.buckets.len(), 2);
        assert_eq!(resp.buckets[0].file_count, 2);
        assert_eq!(resp.buckets[1].file_count, 1);
    }

    #[test]
    fn since_filter_excludes_earlier_buckets() {
        let conn = make_conn();
        seed_file(
            &conn,
            "C:\\r\\a.txt",
            "C:\\r",
            "a.txt",
            100,
            Some(DAY1),
            "Document",
        );
        seed_file(
            &conn,
            "C:\\r\\b.txt",
            "C:\\r",
            "b.txt",
            200,
            Some(DAY2),
            "Document",
        );

        let resp = get_timeline(&conn, Granularity::Day, Some(DAY2 - 10)).unwrap();
        assert_eq!(resp.buckets.len(), 1);
        assert_eq!(resp.buckets[0].file_count, 1);
    }

    #[test]
    fn files_with_no_created_at_are_excluded() {
        let conn = make_conn();
        seed_file(
            &conn,
            "C:\\r\\a.txt",
            "C:\\r",
            "a.txt",
            100,
            None,
            "Document",
        );

        let resp = get_timeline(&conn, Granularity::Day, None).unwrap();
        assert!(resp.buckets.is_empty());
    }

    #[test]
    fn screenshot_burst_needs_the_threshold_met_same_day() {
        let conn = make_conn();
        for i in 0..5 {
            seed_file(
                &conn,
                &format!("C:\\r\\Screenshot {i}.png"),
                "C:\\r",
                &format!("Screenshot {i}.png"),
                1_000,
                Some(DAY1 + i64::from(i) * 10),
                "Image",
            );
        }
        // A non-screenshot image the same day must not count toward the burst.
        seed_file(
            &conn,
            "C:\\r\\vacation.jpg",
            "C:\\r",
            "vacation.jpg",
            5_000,
            Some(DAY1),
            "Image",
        );
        // A screenshot-named file that is not an image (misclassified extension) must not count.
        seed_file(
            &conn,
            "C:\\r\\Screenshot notes.txt",
            "C:\\r",
            "Screenshot notes.txt",
            50,
            Some(DAY1),
            "Document",
        );

        let bursts = screenshot_bursts(&conn, 5, 10).unwrap();
        assert_eq!(bursts.len(), 1);
        assert_eq!(bursts[0].file_count, 5);
        assert_eq!(bursts[0].folder, None);
        assert_eq!(bursts[0].sample.len(), 5);

        let too_strict = screenshot_bursts(&conn, 6, 10).unwrap();
        assert!(too_strict.is_empty());
    }

    #[test]
    fn project_burst_requires_same_folder_and_same_day() {
        let conn = make_conn();
        for i in 0..4 {
            seed_file(
                &conn,
                &format!("C:\\proj\\file{i}.rs"),
                "C:\\proj",
                &format!("file{i}.rs"),
                100,
                Some(DAY1 + i64::from(i)),
                "Code",
            );
        }
        // Same day, different folder: must not merge into the C:\proj burst.
        seed_file(
            &conn,
            "C:\\other\\x.txt",
            "C:\\other",
            "x.txt",
            100,
            Some(DAY1),
            "Document",
        );
        // Same folder, a different day: must not merge into the DAY1 burst.
        seed_file(
            &conn,
            "C:\\proj\\late.rs",
            "C:\\proj",
            "late.rs",
            100,
            Some(DAY2),
            "Code",
        );

        let bursts = project_bursts(&conn, 4, 10).unwrap();
        assert_eq!(bursts.len(), 1);
        assert_eq!(bursts[0].folder.as_deref(), Some("C:\\proj"));
        assert_eq!(bursts[0].file_count, 4);
    }
}
