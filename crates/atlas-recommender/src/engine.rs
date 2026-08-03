//! Runs every rule against the index and merges the results.
//!
//! Thresholds are fixed, sane defaults rather than user-configurable
//! settings for now: exposing a tuning UI before anyone has asked for one is
//! exactly the kind of speculative surface this project avoids. If a
//! threshold turns out to be wrong for real usage, it is a one-line change.

use rusqlite::Connection;

use crate::rules::{empty_folders, forgotten_installers, old_archives, screenshot_pileups};
use crate::types::Recommendation;

/// Installers not modified in this many days are recommended for review.
pub const INSTALLER_MIN_AGE_DAYS: u32 = 90;
/// Archives not modified in this many days are recommended for review.
pub const ARCHIVE_MIN_AGE_DAYS: u32 = 180;
/// A folder needs at least this many screenshots to be flagged as a pileup.
pub const SCREENSHOT_PILEUP_MIN_COUNT: u32 = 15;

/// Run every rule and return all recommendations, empty-folder and
/// highest-confidence rules first.
pub fn get_recommendations(
    conn: &Connection,
    now_unix: i64,
) -> rusqlite::Result<Vec<Recommendation>> {
    let mut all = Vec::new();
    all.extend(empty_folders(conn)?);
    all.extend(forgotten_installers(
        conn,
        now_unix,
        INSTALLER_MIN_AGE_DAYS,
    )?);
    all.extend(old_archives(conn, now_unix, ARCHIVE_MIN_AGE_DAYS)?);
    all.extend(screenshot_pileups(conn, SCREENSHOT_PILEUP_MIN_COUNT)?);
    Ok(all)
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_db::{apply_migrations, open_in_memory};

    #[test]
    fn returns_empty_vec_on_a_fresh_index() {
        let mut conn = open_in_memory().expect("open");
        apply_migrations(&mut conn).expect("migrate");
        let recs = get_recommendations(&conn, 1_000_000).unwrap();
        assert!(recs.is_empty());
    }
}
