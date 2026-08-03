//! Individual rule definitions.
//!
//! Each rule is a pure read against the index: it never touches the
//! filesystem and never mutates anything. A rule returns zero or more
//! `Recommendation`s; running it twice on an unchanged index returns the
//! same result.
//!
//! Two rules from the original roadmap sketch are deliberately not here yet:
//! see `docs/DECISIONS/0007-recommender-rule-scope.md` for why "stale
//! node_modules" and "broken shortcuts" need more than a SQL query against
//! the current schema and are deferred rather than half-built.

use rusqlite::Connection;

use crate::types::{Confidence, Recommendation, RecommendationItem};

const SECONDS_PER_DAY: i64 = 86_400;

/// Folders with zero live children. Always safe to remove: an empty folder
/// carries no data, so this is the highest-confidence rule in the set.
pub fn empty_folders(conn: &Connection) -> rusqlite::Result<Vec<Recommendation>> {
    let mut stmt = conn.prepare(
        "SELECT f.path, f.name, f.modified_at
         FROM files f
         WHERE f.is_dir = 1 AND f.removed_at IS NULL
           AND NOT EXISTS (
               SELECT 1 FROM files c
               WHERE c.parent = f.path AND c.removed_at IS NULL
           )
         ORDER BY f.path",
    )?;
    let items: Vec<RecommendationItem> = stmt
        .query_map([], |r| {
            Ok(RecommendationItem {
                path: r.get(0)?,
                name: r.get(1)?,
                size_bytes: 0,
                modified_at: r.get(2)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    if items.is_empty() {
        return Ok(vec![]);
    }

    Ok(vec![Recommendation {
        kind: "empty_folders".to_string(),
        title: format!("{} empty folders", items.len()),
        explanation: "These folders have nothing in them. Removing them is always safe."
            .to_string(),
        confidence: Confidence::High,
        total_bytes: 0,
        items,
    }])
}

/// Installer files that have not been touched in a long time.
///
/// `.exe`, `.msi`, and similar, tagged `Installer` by the classifier. Once
/// you have run an installer, the installer file itself is rarely needed
/// again.
pub fn forgotten_installers(
    conn: &Connection,
    now_unix: i64,
    min_age_days: u32,
) -> rusqlite::Result<Vec<Recommendation>> {
    let cutoff = now_unix - i64::from(min_age_days) * SECONDS_PER_DAY;
    let mut stmt = conn.prepare(
        "SELECT path, name, size_bytes, modified_at
         FROM files
         WHERE category = 'Installer' AND removed_at IS NULL AND is_dir = 0
           AND modified_at IS NOT NULL AND modified_at < ?1
         ORDER BY size_bytes DESC",
    )?;
    let items: Vec<RecommendationItem> = stmt
        .query_map(rusqlite::params![cutoff], row_to_item)?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    if items.is_empty() {
        return Ok(vec![]);
    }

    let total_bytes: i64 = items.iter().map(|i| i.size_bytes).sum();
    Ok(vec![Recommendation {
        kind: "forgotten_installers".to_string(),
        title: format!("{} installers you probably already ran", items.len()),
        explanation: format!(
            "Not modified in at least {min_age_days} days. Once an installer has been run, the file itself is usually safe to remove."
        ),
        confidence: Confidence::Medium,
        total_bytes,
        items,
    }])
}

/// Archives (`.zip`, `.rar`, and similar) that have not been touched in a
/// long time. Lower confidence than installers: some archives are
/// intentionally kept as long-term backups.
pub fn old_archives(
    conn: &Connection,
    now_unix: i64,
    min_age_days: u32,
) -> rusqlite::Result<Vec<Recommendation>> {
    let cutoff = now_unix - i64::from(min_age_days) * SECONDS_PER_DAY;
    let mut stmt = conn.prepare(
        "SELECT path, name, size_bytes, modified_at
         FROM files
         WHERE category = 'Archive' AND removed_at IS NULL AND is_dir = 0
           AND modified_at IS NOT NULL AND modified_at < ?1
         ORDER BY size_bytes DESC",
    )?;
    let items: Vec<RecommendationItem> = stmt
        .query_map(rusqlite::params![cutoff], row_to_item)?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    if items.is_empty() {
        return Ok(vec![]);
    }

    let total_bytes: i64 = items.iter().map(|i| i.size_bytes).sum();
    Ok(vec![Recommendation {
        kind: "old_archives".to_string(),
        title: format!("{} archives untouched for a while", items.len()),
        explanation: format!(
            "Not modified in at least {min_age_days} days. Some archives are kept as intentional backups, so review these before removing anything."
        ),
        confidence: Confidence::Low,
        total_bytes,
        items,
    }])
}

/// Folders containing a large number of screenshot-named image files. One
/// `Recommendation` per qualifying folder, since "clean up your screenshots"
/// is a per-folder decision, not a single global one.
pub fn screenshot_pileups(
    conn: &Connection,
    min_count: u32,
) -> rusqlite::Result<Vec<Recommendation>> {
    let mut folder_stmt = conn.prepare(
        "SELECT parent, COUNT(*) as cnt
         FROM files
         WHERE category = 'Image' AND removed_at IS NULL AND is_dir = 0
           AND (
               LOWER(name) LIKE 'screenshot%'
               OR LOWER(name) LIKE 'screen shot%'
               OR LOWER(name) LIKE 'screen_shot%'
           )
         GROUP BY parent
         HAVING cnt >= ?1
         ORDER BY cnt DESC",
    )?;
    let folders: Vec<(String, i64)> = folder_stmt
        .query_map(rusqlite::params![min_count], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let mut item_stmt = conn.prepare(
        "SELECT path, name, size_bytes, modified_at
         FROM files
         WHERE category = 'Image' AND removed_at IS NULL AND is_dir = 0 AND parent = ?1
           AND (
               LOWER(name) LIKE 'screenshot%'
               OR LOWER(name) LIKE 'screen shot%'
               OR LOWER(name) LIKE 'screen_shot%'
           )
         ORDER BY name",
    )?;

    let mut recommendations = Vec::with_capacity(folders.len());
    for (parent, count) in folders {
        let items: Vec<RecommendationItem> = item_stmt
            .query_map(rusqlite::params![parent], row_to_item)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let total_bytes: i64 = items.iter().map(|i| i.size_bytes).sum();
        recommendations.push(Recommendation {
            kind: "screenshot_pileup".to_string(),
            title: format!("{count} screenshots in {parent}"),
            explanation:
                "This folder has a lot of screenshots. Worth a look before they pile up further."
                    .to_string(),
            confidence: Confidence::Low,
            total_bytes,
            items,
        });
    }
    Ok(recommendations)
}

fn row_to_item(r: &rusqlite::Row<'_>) -> rusqlite::Result<RecommendationItem> {
    Ok(RecommendationItem {
        path: r.get(0)?,
        name: r.get(1)?,
        size_bytes: r.get(2)?,
        modified_at: r.get(3)?,
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
    fn seed(
        conn: &Connection,
        path: &str,
        parent: &str,
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
                name: path.rsplit(['\\', '/']).next().unwrap_or(path).to_string(),
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
    fn empty_folders_finds_folders_with_no_children() {
        let conn = make_conn();
        seed(&conn, "C:\\r\\empty", "C:\\r", 0, Some(1), "Folder", true);
        seed(&conn, "C:\\r\\full", "C:\\r", 0, Some(1), "Folder", true);
        seed(
            &conn,
            "C:\\r\\full\\a.txt",
            "C:\\r\\full",
            10,
            Some(1),
            "Document",
            false,
        );

        let recs = empty_folders(&conn).unwrap();
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].items.len(), 1);
        assert_eq!(recs[0].items[0].path, "C:\\r\\empty");
        assert_eq!(recs[0].confidence, Confidence::High);
    }

    #[test]
    fn empty_folders_returns_nothing_when_none_exist() {
        let conn = make_conn();
        seed(&conn, "C:\\r\\full", "C:\\r", 0, Some(1), "Folder", true);
        seed(
            &conn,
            "C:\\r\\full\\a.txt",
            "C:\\r\\full",
            10,
            Some(1),
            "Document",
            false,
        );
        assert!(empty_folders(&conn).unwrap().is_empty());
    }

    #[test]
    fn forgotten_installers_filters_by_category_and_age() {
        let conn = make_conn();
        let now = 100_000_000i64;
        let old = now - 200 * SECONDS_PER_DAY;
        let recent = now - 10 * SECONDS_PER_DAY;
        seed(
            &conn,
            "C:\\r\\setup_old.exe",
            "C:\\r",
            1000,
            Some(old),
            "Installer",
            false,
        );
        seed(
            &conn,
            "C:\\r\\setup_new.exe",
            "C:\\r",
            1000,
            Some(recent),
            "Installer",
            false,
        );
        seed(
            &conn,
            "C:\\r\\doc.pdf",
            "C:\\r",
            1000,
            Some(old),
            "Document",
            false,
        );

        let recs = forgotten_installers(&conn, now, 90).unwrap();
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].items.len(), 1);
        assert_eq!(recs[0].items[0].name, "setup_old.exe");
        assert_eq!(recs[0].total_bytes, 1000);
    }

    #[test]
    fn old_archives_filters_by_category_and_age() {
        let conn = make_conn();
        let now = 100_000_000i64;
        let old = now - 200 * SECONDS_PER_DAY;
        seed(
            &conn,
            "C:\\r\\backup.zip",
            "C:\\r",
            5000,
            Some(old),
            "Archive",
            false,
        );

        let recs = old_archives(&conn, now, 90).unwrap();
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].confidence, Confidence::Low);
        assert_eq!(recs[0].items[0].name, "backup.zip");
    }

    #[test]
    fn screenshot_pileups_groups_by_folder_above_threshold() {
        let conn = make_conn();
        for i in 0..5 {
            seed(
                &conn,
                &format!("C:\\r\\shots\\Screenshot_{i}.png"),
                "C:\\r\\shots",
                100,
                Some(1),
                "Image",
                false,
            );
        }
        // Only one screenshot here: should not qualify at threshold 3.
        seed(
            &conn,
            "C:\\r\\other\\Screenshot_x.png",
            "C:\\r\\other",
            100,
            Some(1),
            "Image",
            false,
        );

        let recs = screenshot_pileups(&conn, 3).unwrap();
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].items.len(), 5);
        assert!(recs[0].title.contains("C:\\r\\shots"));
    }

    #[test]
    fn screenshot_pileups_ignores_non_screenshot_images() {
        let conn = make_conn();
        for i in 0..5 {
            seed(
                &conn,
                &format!("C:\\r\\photos\\vacation_{i}.jpg"),
                "C:\\r\\photos",
                100,
                Some(1),
                "Image",
                false,
            );
        }
        assert!(screenshot_pileups(&conn, 3).unwrap().is_empty());
    }
}
