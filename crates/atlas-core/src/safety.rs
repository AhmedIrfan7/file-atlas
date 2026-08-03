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
///
/// One list per OS: Windows, macOS, and Linux each own their filesystem
/// layout, so there is no single prefix set that means anything on all
/// three. See ADR 0010 for what is deliberately left out (per-user trash
/// folders need a runtime home-directory lookup, not a `const` list; that is
/// layered in separately by `seed_defaults` below).
#[cfg(windows)]
const DEFAULT_PROTECTED_PREFIXES: &[(&str, &str)] = &[
    ("C:\\Windows", "Windows system directory"),
    ("C:\\Program Files", "installed applications"),
    ("C:\\Program Files (x86)", "installed applications"),
    ("C:\\ProgramData", "shared application data"),
    ("C:\\$Recycle.Bin", "the Recycle Bin itself"),
    ("C:\\System Volume Information", "Windows system directory"),
];

#[cfg(target_os = "macos")]
const DEFAULT_PROTECTED_PREFIXES: &[(&str, &str)] = &[
    ("/System", "macOS system directory"),
    ("/Library", "shared application support data"),
    ("/private", "macOS system directory"),
    ("/usr", "macOS system directory"),
    ("/bin", "macOS system directory"),
    ("/sbin", "macOS system directory"),
    ("/Applications", "installed applications"),
];

#[cfg(target_os = "linux")]
const DEFAULT_PROTECTED_PREFIXES: &[(&str, &str)] = &[
    ("/usr", "Linux system directory"),
    ("/bin", "Linux system directory"),
    ("/sbin", "Linux system directory"),
    ("/lib", "Linux system directory"),
    ("/lib64", "Linux system directory"),
    ("/etc", "Linux system configuration"),
    ("/boot", "Linux boot files"),
    ("/proc", "Linux virtual filesystem"),
    ("/sys", "Linux virtual filesystem"),
    ("/dev", "Linux device files"),
    ("/opt", "installed applications"),
    ("/snap", "installed snap applications"),
];

#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
const DEFAULT_PROTECTED_PREFIXES: &[(&str, &str)] = &[];

/// Insert the default protected prefixes if they are not already present.
///
/// Safe (and intended) to call on every startup. This is self-healing by
/// design: if a default row is missing, whether because this is a first run
/// or because something deleted it, it comes back. Protected paths guard
/// against destroying the operating system, so "a row went missing" must
/// never quietly downgrade to "that system directory is unprotected" -
/// re-seeding is the safer failure mode. Rows that still exist (including
/// ones a user has edited the reason/timestamp on) are left untouched.
///
/// Also seeds the current user's OS trash folder (macOS `~/.Trash`, Linux
/// `~/.local/share/Trash`) when the home directory can be resolved at
/// runtime, since that is not knowable at compile time the way the rest of
/// `DEFAULT_PROTECTED_PREFIXES` is. Windows does not need this: its trash is
/// the single machine-wide `C:\$Recycle.Bin` already in the const list.
pub fn seed_defaults(conn: &Connection, now: i64) -> rusqlite::Result<()> {
    for (path, reason) in DEFAULT_PROTECTED_PREFIXES {
        conn.execute(
            "INSERT OR IGNORE INTO protected_paths (path, reason, added_at) VALUES (?1, ?2, ?3)",
            rusqlite::params![path, reason, now],
        )?;
    }
    if let Some(trash) = user_trash_dir() {
        conn.execute(
            "INSERT OR IGNORE INTO protected_paths (path, reason, added_at) VALUES (?1, ?2, ?3)",
            rusqlite::params![trash.to_string_lossy(), "the user's own trash folder", now],
        )?;
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn user_trash_dir() -> Option<std::path::PathBuf> {
    dirs::home_dir().map(|h| h.join(".Trash"))
}

#[cfg(target_os = "linux")]
fn user_trash_dir() -> Option<std::path::PathBuf> {
    dirs::home_dir().map(|h| h.join(".local/share/Trash"))
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
const fn user_trash_dir() -> Option<std::path::PathBuf> {
    None
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
                .find(|(prefix, _)| path_has_prefix(path, prefix));
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

/// Windows and macOS filesystems are case-insensitive by default, so a
/// protected prefix must match regardless of case. Linux filesystems are
/// case-sensitive, so `/USR` and `/usr` are genuinely different paths there
/// and must not be conflated.
#[cfg(not(target_os = "linux"))]
fn path_has_prefix(haystack: &str, needle: &str) -> bool {
    if haystack.len() < needle.len() {
        return false;
    }
    haystack
        .chars()
        .zip(needle.chars())
        .all(|(a, b)| a.eq_ignore_ascii_case(&b))
}

#[cfg(target_os = "linux")]
fn path_has_prefix(haystack: &str, needle: &str) -> bool {
    haystack.starts_with(needle)
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

    /// One real default prefix to test against, taken from the actual
    /// per-OS list rather than hardcoded, so these tests exercise whatever
    /// this platform's real defaults are instead of assuming Windows paths.
    fn a_protected_prefix() -> &'static str {
        DEFAULT_PROTECTED_PREFIXES[0].0
    }

    #[test]
    fn seeding_twice_does_not_duplicate() {
        let conn = make_conn();
        seed_defaults(&conn, 1).unwrap();
        let count_after_first: i64 = conn
            .query_row("SELECT COUNT(*) FROM protected_paths", [], |r| r.get(0))
            .unwrap();
        seed_defaults(&conn, 2).unwrap();
        let count_after_second: i64 = conn
            .query_row("SELECT COUNT(*) FROM protected_paths", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count_after_first, count_after_second);
    }

    #[test]
    fn blocks_paths_under_protected_prefixes() {
        let conn = make_conn();
        seed_defaults(&conn, 1).unwrap();
        let prefix = a_protected_prefix();
        let decisions = check_paths(&conn, &[format!("{prefix}/some/inner/file")]).unwrap();
        assert!(!decisions[0].allowed);
        assert!(decisions[0].reason.is_some());
    }

    #[test]
    fn allows_paths_outside_protected_prefixes() {
        let conn = make_conn();
        seed_defaults(&conn, 1).unwrap();
        let decisions = check_paths(
            &conn,
            &["definitely-not-a-protected-path/notes.txt".to_string()],
        )
        .unwrap();
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
        let prefix = a_protected_prefix();
        conn.execute("DELETE FROM protected_paths WHERE path = ?1", [prefix])
            .unwrap();
        seed_defaults(&conn, 2).unwrap();
        let decisions = check_paths(&conn, &[format!("{prefix}/thing.txt")]).unwrap();
        assert!(!decisions[0].allowed, "default protection should self-heal");
    }

    #[test]
    fn preserves_input_order() {
        let conn = make_conn();
        seed_defaults(&conn, 1).unwrap();
        let prefix = a_protected_prefix();
        let decisions = check_paths(
            &conn,
            &[
                "unrelated/a.txt".to_string(),
                format!("{prefix}/b.txt"),
                "unrelated/c.txt".to_string(),
            ],
        )
        .unwrap();
        assert_eq!(decisions.len(), 3);
        assert!(decisions[0].allowed);
        assert!(!decisions[1].allowed);
        assert!(decisions[2].allowed);
    }

    #[cfg(not(target_os = "linux"))]
    #[test]
    fn prefix_matching_is_case_insensitive_on_windows_and_macos() {
        let conn = make_conn();
        seed_defaults(&conn, 1).unwrap();
        let prefix = a_protected_prefix();
        let shouted = prefix.to_uppercase();
        let decisions = check_paths(&conn, &[format!("{shouted}/thing")]).unwrap();
        assert!(!decisions[0].allowed);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn prefix_matching_is_case_sensitive_on_linux() {
        let conn = make_conn();
        seed_defaults(&conn, 1).unwrap();
        let prefix = a_protected_prefix();
        let shouted = prefix.to_uppercase();
        // Different case means a genuinely different path on a case-sensitive
        // filesystem, so this must NOT be treated as protected.
        let decisions = check_paths(&conn, &[format!("{shouted}/thing")]).unwrap();
        assert!(decisions[0].allowed);
    }
}
