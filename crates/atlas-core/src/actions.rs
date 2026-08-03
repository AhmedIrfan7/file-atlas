//! Executes destructive operations: trash and restore.
//!
//! This is the last stage of the safety pipeline documented in
//! `docs/ARCHITECTURE.md`: `request -> guardrails -> preview -> confirm ->
//! execute -> action_log -> undo affordance`. By the time a path reaches
//! `trash_paths`, the UI has already shown a preview and gotten explicit
//! confirmation; this module's own job is just guardrails plus execution
//! plus writing a durable, restorable record of what happened.
//!
//! Every successful trash writes one `actions_log` row before it is
//! considered done, and marks the file `removed_at` in the index so scans
//! and views agree the file is gone. `restore_action` reverses exactly one
//! such row.

use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use atlas_db::ActionRow;
use atlas_platform::{PlatformFs, TrashHandle};

use crate::safety::check_paths;

/// Outcome of attempting to trash one path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrashOutcome {
    pub path: String,
    pub ok: bool,
    pub reason: Option<String>,
    pub action_id: Option<i64>,
}

/// Outcome of restoring one previously trashed path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestoreOutcome {
    pub action_id: i64,
    pub ok: bool,
    pub reason: Option<String>,
    pub restored_path: Option<String>,
}

/// Send every path in `paths` to the OS trash, guardrails permitting.
///
/// Paths blocked by `safety::check_paths` are reported with `ok: false` and
/// never reach the platform layer. Each path that is actually trashed gets
/// its own `actions_log` row and has `files.removed_at` set, independent of
/// whether other paths in the same call succeed or fail.
pub fn trash_paths(
    conn: &Connection,
    platform: &dyn PlatformFs,
    paths: &[String],
    now: i64,
) -> rusqlite::Result<Vec<TrashOutcome>> {
    let decisions = check_paths(conn, paths)?;
    let mut outcomes = Vec::with_capacity(paths.len());

    for decision in decisions {
        if !decision.allowed {
            outcomes.push(TrashOutcome {
                path: decision.path,
                ok: false,
                reason: decision.reason,
                action_id: None,
            });
            continue;
        }

        match platform.send_to_trash(std::path::Path::new(&decision.path)) {
            Ok(handle) => {
                let action_id = log_trash(conn, &decision.path, &handle, now)?;
                mark_removed(conn, &decision.path, now)?;
                outcomes.push(TrashOutcome {
                    path: decision.path,
                    ok: true,
                    reason: None,
                    action_id: Some(action_id),
                });
            }
            Err(err) => outcomes.push(TrashOutcome {
                path: decision.path,
                ok: false,
                reason: Some(err.to_string()),
                action_id: None,
            }),
        }
    }

    Ok(outcomes)
}

/// Restore the file trashed by the `actions_log` row `action_id`. No-op on
/// the index if the row is not a reversible trash action.
pub fn restore_action(
    conn: &Connection,
    platform: &dyn PlatformFs,
    action_id: i64,
    now: i64,
) -> rusqlite::Result<RestoreOutcome> {
    let Some((path_from, handle)) = load_trash_action(conn, action_id)? else {
        return Ok(RestoreOutcome {
            action_id,
            ok: false,
            reason: Some("no reversible trash action with that id".to_string()),
            restored_path: None,
        });
    };

    match platform.restore_from_trash(&handle) {
        Ok(restored) => {
            let restored_str = restored.to_string_lossy().into_owned();
            clear_removed(conn, &path_from, now)?;
            log_restore(conn, &path_from, &restored_str, now)?;
            Ok(RestoreOutcome {
                action_id,
                ok: true,
                reason: None,
                restored_path: Some(restored_str),
            })
        }
        Err(err) => Ok(RestoreOutcome {
            action_id,
            ok: false,
            reason: Some(err.to_string()),
            restored_path: None,
        }),
    }
}

/// The `limit` most recent trash actions, newest first. Used to power an
/// undo / "recently deleted" panel.
pub fn list_recent_trash_actions(
    conn: &Connection,
    limit: u32,
) -> rusqlite::Result<Vec<ActionRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, ts, op, path_from, path_to, metadata, reversible, undo_ref
         FROM actions_log
         WHERE op = 'trash'
         ORDER BY ts DESC
         LIMIT ?1",
    )?;
    let rows = stmt.query_map(rusqlite::params![limit], |r| {
        Ok(ActionRow {
            id: r.get(0)?,
            ts: r.get(1)?,
            op: r.get(2)?,
            path_from: r.get(3)?,
            path_to: r.get(4)?,
            metadata: r.get(5)?,
            reversible: r.get::<_, i64>(6)? != 0,
            undo_ref: r.get(7)?,
        })
    })?;
    rows.collect()
}

#[derive(Serialize, Deserialize)]
struct TrashMetadata {
    kind: String,
    token: String,
}

fn log_trash(
    conn: &Connection,
    path: &str,
    handle: &TrashHandle,
    now: i64,
) -> rusqlite::Result<i64> {
    let metadata = serde_json::to_string(&TrashMetadata {
        kind: handle.kind.clone(),
        token: handle.token.clone(),
    })
    .unwrap_or_default();
    conn.execute(
        "INSERT INTO actions_log (ts, op, path_from, path_to, metadata, reversible, undo_ref)
         VALUES (?1, 'trash', ?2, NULL, ?3, 1, ?4)",
        rusqlite::params![now, path, metadata, handle.token],
    )?;
    Ok(conn.last_insert_rowid())
}

fn log_restore(
    conn: &Connection,
    path_from: &str,
    path_to: &str,
    now: i64,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO actions_log (ts, op, path_from, path_to, metadata, reversible, undo_ref)
         VALUES (?1, 'restore', ?2, ?3, NULL, 0, NULL)",
        rusqlite::params![now, path_from, path_to],
    )?;
    Ok(())
}

fn load_trash_action(
    conn: &Connection,
    action_id: i64,
) -> rusqlite::Result<Option<(String, TrashHandle)>> {
    let row: Option<(String, String, bool)> = conn
        .query_row(
            "SELECT path_from, metadata, reversible FROM actions_log
             WHERE id = ?1 AND op = 'trash'",
            rusqlite::params![action_id],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, i64>(2)? != 0,
                ))
            },
        )
        .optional()?;

    let Some((path_from, metadata, reversible)) = row else {
        return Ok(None);
    };
    if !reversible {
        return Ok(None);
    }
    let parsed: TrashMetadata = match serde_json::from_str(&metadata) {
        Ok(m) => m,
        Err(_) => return Ok(None),
    };
    Ok(Some((
        path_from,
        TrashHandle {
            kind: parsed.kind,
            token: parsed.token,
        },
    )))
}

fn mark_removed(conn: &Connection, path: &str, now: i64) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE files SET removed_at = ?1 WHERE path = ?2",
        rusqlite::params![now, path],
    )?;
    Ok(())
}

fn clear_removed(conn: &Connection, path: &str, now: i64) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE files SET removed_at = NULL, last_seen = ?1 WHERE path = ?2",
        rusqlite::params![now, path],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_db::queries::{upsert_file, upsert_volume};
    use atlas_db::{apply_migrations, open_in_memory, FileRow, VolumeRow};
    use atlas_platform::{PlatformError, Volume};
    use std::sync::Mutex;

    fn make_conn() -> Connection {
        let mut c = open_in_memory().expect("open");
        apply_migrations(&mut c).expect("migrate");
        c
    }

    fn seed_file(conn: &Connection, path: &str) {
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
                size_bytes: 10,
                created_at: Some(1),
                modified_at: Some(1),
                accessed_at: Some(1),
                hash_blake3: None,
                hash_size: None,
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

    /// A fake `PlatformFs` so these tests never touch the real Recycle Bin.
    /// Trashing just records the path; restoring hands the same path back.
    #[derive(Debug, Default)]
    struct FakePlatform {
        trashed: Mutex<Vec<String>>,
        fail_send: Mutex<Option<String>>,
    }

    impl PlatformFs for FakePlatform {
        fn list_volumes(&self) -> atlas_platform::Result<Vec<Volume>> {
            Ok(vec![])
        }
        fn is_hidden(&self, _path: &std::path::Path) -> atlas_platform::Result<bool> {
            Ok(false)
        }
        fn is_system(&self, _path: &std::path::Path) -> atlas_platform::Result<bool> {
            Ok(false)
        }
        fn send_to_trash(&self, path: &std::path::Path) -> atlas_platform::Result<TrashHandle> {
            let fail_message = self.fail_send.lock().unwrap().clone();
            if let Some(msg) = fail_message {
                return Err(PlatformError::Api(msg));
            }
            let path_str = path.to_string_lossy().into_owned();
            self.trashed.lock().unwrap().push(path_str.clone());
            Ok(TrashHandle {
                kind: "os-trash".to_string(),
                token: serde_json::to_string(&path_str).unwrap(),
            })
        }
        fn restore_from_trash(
            &self,
            handle: &TrashHandle,
        ) -> atlas_platform::Result<std::path::PathBuf> {
            let path_str: String = serde_json::from_str(&handle.token).unwrap();
            Ok(std::path::PathBuf::from(path_str))
        }
    }

    #[test]
    fn trash_paths_logs_action_and_marks_removed() {
        let conn = make_conn();
        seed_file(&conn, "C:\\Users\\me\\Desktop\\dup.txt");
        crate::safety::seed_defaults(&conn, 0).unwrap();
        let platform = FakePlatform::default();

        let outcomes = trash_paths(
            &conn,
            &platform,
            &["C:\\Users\\me\\Desktop\\dup.txt".to_string()],
            100,
        )
        .unwrap();

        assert!(outcomes[0].ok);
        assert!(outcomes[0].action_id.is_some());

        let removed_at: Option<i64> = conn
            .query_row(
                "SELECT removed_at FROM files WHERE path = ?1",
                rusqlite::params!["C:\\Users\\me\\Desktop\\dup.txt"],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(removed_at, Some(100));
    }

    #[test]
    fn trash_paths_blocks_protected_paths() {
        let conn = make_conn();
        crate::safety::seed_defaults(&conn, 0).unwrap();
        let platform = FakePlatform::default();

        let outcomes = trash_paths(
            &conn,
            &platform,
            &["C:\\Windows\\system32\\notepad.exe".to_string()],
            100,
        )
        .unwrap();

        assert!(!outcomes[0].ok);
        assert!(outcomes[0].action_id.is_none());
        assert!(platform.trashed.lock().unwrap().is_empty());
    }

    #[test]
    fn trash_paths_reports_platform_errors_without_logging() {
        let conn = make_conn();
        seed_file(&conn, "C:\\Users\\me\\Desktop\\locked.txt");
        crate::safety::seed_defaults(&conn, 0).unwrap();
        let platform = FakePlatform::default();
        *platform.fail_send.lock().unwrap() = Some("access denied".to_string());

        let outcomes = trash_paths(
            &conn,
            &platform,
            &["C:\\Users\\me\\Desktop\\locked.txt".to_string()],
            100,
        )
        .unwrap();

        assert!(!outcomes[0].ok);
        assert!(outcomes[0]
            .reason
            .as_ref()
            .unwrap()
            .contains("access denied"));

        let removed_at: Option<i64> = conn
            .query_row(
                "SELECT removed_at FROM files WHERE path = ?1",
                rusqlite::params!["C:\\Users\\me\\Desktop\\locked.txt"],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(removed_at, None, "failed trash must not mark removed");
    }

    #[test]
    fn restore_action_reverses_a_trash_and_clears_removed() {
        let conn = make_conn();
        seed_file(&conn, "C:\\Users\\me\\Desktop\\dup.txt");
        crate::safety::seed_defaults(&conn, 0).unwrap();
        let platform = FakePlatform::default();

        let outcomes = trash_paths(
            &conn,
            &platform,
            &["C:\\Users\\me\\Desktop\\dup.txt".to_string()],
            100,
        )
        .unwrap();
        let action_id = outcomes[0].action_id.unwrap();

        let restore = restore_action(&conn, &platform, action_id, 200).unwrap();
        assert!(restore.ok);
        assert_eq!(
            restore.restored_path.as_deref(),
            Some("C:\\Users\\me\\Desktop\\dup.txt")
        );

        let removed_at: Option<i64> = conn
            .query_row(
                "SELECT removed_at FROM files WHERE path = ?1",
                rusqlite::params!["C:\\Users\\me\\Desktop\\dup.txt"],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(removed_at, None);
    }

    #[test]
    fn restore_action_on_unknown_id_reports_failure_not_panic() {
        let conn = make_conn();
        let platform = FakePlatform::default();
        let restore = restore_action(&conn, &platform, 999, 100).unwrap();
        assert!(!restore.ok);
    }

    #[test]
    fn list_recent_trash_actions_orders_newest_first() {
        let conn = make_conn();
        seed_file(&conn, "C:\\Users\\me\\Desktop\\a.txt");
        seed_file(&conn, "C:\\Users\\me\\Desktop\\b.txt");
        crate::safety::seed_defaults(&conn, 0).unwrap();
        let platform = FakePlatform::default();

        trash_paths(
            &conn,
            &platform,
            &["C:\\Users\\me\\Desktop\\a.txt".to_string()],
            100,
        )
        .unwrap();
        trash_paths(
            &conn,
            &platform,
            &["C:\\Users\\me\\Desktop\\b.txt".to_string()],
            200,
        )
        .unwrap();

        let recent = list_recent_trash_actions(&conn, 10).unwrap();
        assert_eq!(recent.len(), 2);
        assert_eq!(
            recent[0].path_from.as_deref(),
            Some("C:\\Users\\me\\Desktop\\b.txt")
        );
        assert_eq!(
            recent[1].path_from.as_deref(),
            Some("C:\\Users\\me\\Desktop\\a.txt")
        );
    }
}
