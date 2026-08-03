//! Guardrails consulted before any destructive operation.
//!
//! This is deliberately narrow: it answers "is this path protected?" against
//! the `protected_paths` table (migration 0001). It does not know about
//! trash, undo, or the UI confirmation flow; those are `actions`'s job. See
//! ADR 0004 for why this is a separate concern from `SkipRules`.

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

/// Default protected prefixes seeded on first run. Mirrors
/// `SkipRules::default()`'s system paths, but this list is enforced at
/// delete/move time rather than at scan time, and is user-editable in the
/// `protected_paths` table (seeding never overwrites existing rows).
const DEFAULT_PROTECTED_PREFIXES: &[(&str, &str)] = &[
    ("C:\\Windows", "Windows system directory"),
    ("C:\\Program Files", "installed applications"),
    ("C:\\Program Files (x86)", "installed applications"),
    ("C:\\ProgramData", "shared application data"),
    ("C:\\$Recycle.Bin", "the Recycle Bin itself"),
    ("C:\\System Volume Information", "Windows system directory"),
];

/// Insert the default protected prefixes if they are not already present.
///
/// Safe (and intended) to call on every startup. This is self-healing by
/// design: if a default row is missing, whether because this is a first run
/// or because something deleted it, it comes back. Protected paths guard
/// against destroying the operating system, so "a row went missing" must
/// never quietly downgrade to "that system directory is unprotected" -
/// re-seeding is the safer failure mode. Rows that still exist (including
/// ones a user has edited the reason/timestamp on) are left untouched.
pub fn seed_defaults(conn: &Connection, now: i64) -> rusqlite::Result<()> {
    for (path, reason) in DEFAULT_PROTECTED_PREFIXES {
        conn.execute(
            "INSERT OR IGNORE INTO protected_paths (path, reason, added_at) VALUES (?1, ?2, ?3)",
            rusqlite::params![path, reason, now],
        )?;
    }
    Ok(())
}

/// The outcome of a guardrail check for one path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuardrailDecision {
    pub path: String,
    pub allowed: bool,
    pub reason: Option<String>,
}

/// Check every path in `paths` against the protected-path list. Returns one
/// decision per input path, in the same order.
pub fn check_paths(
    conn: &Connection,
    paths: &[String],
) -> rusqlite::Result<Vec<GuardrailDecision>> {
    let prefixes = load_protected_prefixes(conn)?;
    Ok(paths
        .iter()
        .map(|path| {
            let blocked = prefixes
                .iter()
                .find(|(prefix, _)| starts_with_ci(path, prefix));
            match blocked {
                Some((_, reason)) => GuardrailDecision {
                    path: path.clone(),
                    allowed: false,
                    reason: Some(reason.clone()),
                },
                None => GuardrailDecision {
                    path: path.clone(),
                    allowed: true,
                    reason: None,
                },
            }
        })
        .collect())
}

fn load_protected_prefixes(conn: &Connection) -> rusqlite::Result<Vec<(String, String)>> {
    let mut stmt = conn.prepare("SELECT path, reason FROM protected_paths")?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
    rows.collect()
}

fn starts_with_ci(haystack: &str, needle: &str) -> bool {
    if haystack.len() < needle.len() {
        return false;
    }
    haystack
        .chars()
        .zip(needle.chars())
        .all(|(a, b)| a.eq_ignore_ascii_case(&b))
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_db::{apply_migrations, open_in_memory};

    fn make_conn() -> Connection {
        let mut c = open_in_memory().expect("open");
        apply_migrations(&mut c).expect("migrate");
        c
    }

    #[test]
    fn seeding_twice_does_not_duplicate() {
        let conn = make_conn();
        seed_defaults(&conn, 1).unwrap();
        seed_defaults(&conn, 2).unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM protected_paths", [], |r| r.get(0))
            .unwrap();
        assert_eq!(
            count,
            i64::try_from(DEFAULT_PROTECTED_PREFIXES.len()).unwrap()
        );
    }

    #[test]
    fn blocks_paths_under_protected_prefixes_case_insensitively() {
        let conn = make_conn();
        seed_defaults(&conn, 1).unwrap();
        let decisions =
            check_paths(&conn, &["c:\\windows\\system32\\notepad.exe".to_string()]).unwrap();
        assert!(!decisions[0].allowed);
        assert!(decisions[0].reason.is_some());
    }

    #[test]
    fn allows_paths_outside_protected_prefixes() {
        let conn = make_conn();
        seed_defaults(&conn, 1).unwrap();
        let decisions =
            check_paths(&conn, &["C:\\Users\\me\\Desktop\\notes.txt".to_string()]).unwrap();
        assert!(decisions[0].allowed);
        assert!(decisions[0].reason.is_none());
    }

    #[test]
    fn seeding_restores_a_deleted_default_because_safety_first() {
        // A missing protection row for a system directory must never
        // silently stay missing: re-seeding heals it back rather than
        // trusting whatever deleted it (accident or bug) was intentional.
        let conn = make_conn();
        seed_defaults(&conn, 1).unwrap();
        conn.execute(
            "DELETE FROM protected_paths WHERE path = 'C:\\ProgramData'",
            [],
        )
        .unwrap();
        seed_defaults(&conn, 2).unwrap();
        let decisions = check_paths(&conn, &["C:\\ProgramData\\thing.txt".to_string()]).unwrap();
        assert!(!decisions[0].allowed, "default protection should self-heal");
    }

    #[test]
    fn preserves_input_order() {
        let conn = make_conn();
        seed_defaults(&conn, 1).unwrap();
        let decisions = check_paths(
            &conn,
            &[
                "C:\\Users\\me\\a.txt".to_string(),
                "C:\\Windows\\b.txt".to_string(),
                "C:\\Users\\me\\c.txt".to_string(),
            ],
        )
        .unwrap();
        assert_eq!(decisions.len(), 3);
        assert!(decisions[0].allowed);
        assert!(!decisions[1].allowed);
        assert!(decisions[2].allowed);
    }
}
