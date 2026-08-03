//! Persistence for user-saved searches.
//!
//! A saved search is just a name plus the raw query text; re-running it
//! means parsing that text again through `parser::parse`. Saving under an
//! existing name replaces it, so the list never accumulates duplicates.

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedSearch {
    pub id: i64,
    pub name: String,
    pub query_text: String,
    pub created_at: i64,
}

/// Save `query_text` under `name`. Replaces any existing saved search with
/// the same name, keeping its original `created_at`.
pub fn save(conn: &Connection, name: &str, query_text: &str, now: i64) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO saved_searches (name, query_text, created_at)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(name) DO UPDATE SET query_text = excluded.query_text",
        params![name, query_text, now],
    )?;
    conn.query_row(
        "SELECT id FROM saved_searches WHERE name = ?1",
        params![name],
        |r| r.get(0),
    )
}

/// All saved searches, most recently created first.
pub fn list(conn: &Connection) -> rusqlite::Result<Vec<SavedSearch>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, query_text, created_at FROM saved_searches ORDER BY created_at DESC",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(SavedSearch {
            id: r.get(0)?,
            name: r.get(1)?,
            query_text: r.get(2)?,
            created_at: r.get(3)?,
        })
    })?;
    rows.collect()
}

/// Delete a saved search by id. No error if it does not exist.
pub fn delete(conn: &Connection, id: i64) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM saved_searches WHERE id = ?1", params![id])?;
    Ok(())
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
    fn save_then_list_roundtrips() {
        let conn = make_conn();
        save(&conn, "Big PDFs", "type:pdf size>10mb", 100).unwrap();
        let all = list(&conn).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].name, "Big PDFs");
        assert_eq!(all[0].query_text, "type:pdf size>10mb");
    }

    #[test]
    fn saving_same_name_twice_replaces_not_duplicates() {
        let conn = make_conn();
        save(&conn, "Recent", "age<7d", 100).unwrap();
        save(&conn, "Recent", "age<30d", 200).unwrap();
        let all = list(&conn).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].query_text, "age<30d");
    }

    #[test]
    fn delete_removes_by_id() {
        let conn = make_conn();
        let id = save(&conn, "Temp", "type:tmp", 100).unwrap();
        delete(&conn, id).unwrap();
        assert!(list(&conn).unwrap().is_empty());
    }

    #[test]
    fn delete_of_missing_id_is_not_an_error() {
        let conn = make_conn();
        assert!(delete(&conn, 999).is_ok());
    }

    #[test]
    fn list_orders_newest_first() {
        let conn = make_conn();
        save(&conn, "First", "a", 100).unwrap();
        save(&conn, "Second", "b", 200).unwrap();
        let all = list(&conn).unwrap();
        assert_eq!(all[0].name, "Second");
        assert_eq!(all[1].name, "First");
    }
}
