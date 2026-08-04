//! AI configuration, stored as key-value rows in `ai_settings` (migration
//! 0005) rather than dedicated columns.
//!
//! A handful of optional, rarely-read settings that will likely grow (new
//! provider options, new local models) is exactly what a key-value table is
//! for, versus a schema migration every time a new setting is added.
//!
//! Cloud settings, including the API key, are stored in the same local
//! SQLite database as everything else in File Atlas: this is a single-user,
//! local-first desktop app with no existing encryption-at-rest anywhere in
//! the index, so a plaintext key here is consistent with that trust model,
//! not a new category of risk. OS keychain integration is a real feature of
//! its own (and, like `atlas-platform`, would need a per-OS implementation);
//! it is not attempted here. See ADR 0011.

use std::collections::HashMap;

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

const KEY_CLOUD_ENABLED: &str = "cloud_enabled";
const KEY_CLOUD_BASE_URL: &str = "cloud_base_url";
const KEY_CLOUD_MODEL: &str = "cloud_model";
const KEY_CLOUD_API_KEY: &str = "cloud_api_key";
const KEY_CHAT_MODEL: &str = "chat_model";

/// Every AI-related setting the desktop app's settings screen exposes.
/// `cloud_enabled` defaults to `false`; nothing here is ever assumed true.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiSettings {
    pub cloud_enabled: bool,
    pub cloud_base_url: Option<String>,
    pub cloud_model: Option<String>,
    pub cloud_api_key: Option<String>,
    /// Which installed Ollama model to use for chat / query translation.
    pub chat_model: Option<String>,
}

pub fn get_settings(conn: &Connection) -> rusqlite::Result<AiSettings> {
    let mut stmt = conn.prepare("SELECT key, value FROM ai_settings")?;
    let rows: HashMap<String, String> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;

    Ok(AiSettings {
        cloud_enabled: rows.get(KEY_CLOUD_ENABLED).is_some_and(|v| v == "true"),
        cloud_base_url: rows.get(KEY_CLOUD_BASE_URL).cloned(),
        cloud_model: rows.get(KEY_CLOUD_MODEL).cloned(),
        cloud_api_key: rows.get(KEY_CLOUD_API_KEY).cloned(),
        chat_model: rows.get(KEY_CHAT_MODEL).cloned(),
    })
}

pub fn set_settings(conn: &Connection, settings: &AiSettings) -> rusqlite::Result<()> {
    upsert(
        conn,
        KEY_CLOUD_ENABLED,
        if settings.cloud_enabled {
            "true"
        } else {
            "false"
        },
    )?;
    set_optional(conn, KEY_CLOUD_BASE_URL, settings.cloud_base_url.as_deref())?;
    set_optional(conn, KEY_CLOUD_MODEL, settings.cloud_model.as_deref())?;
    set_optional(conn, KEY_CLOUD_API_KEY, settings.cloud_api_key.as_deref())?;
    set_optional(conn, KEY_CHAT_MODEL, settings.chat_model.as_deref())?;
    Ok(())
}

fn set_optional(conn: &Connection, key: &str, value: Option<&str>) -> rusqlite::Result<()> {
    match value {
        Some(v) if !v.is_empty() => upsert(conn, key, v),
        _ => delete(conn, key),
    }
}

fn upsert(conn: &Connection, key: &str, value: &str) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO ai_settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        rusqlite::params![key, value],
    )?;
    Ok(())
}

fn delete(conn: &Connection, key: &str) -> rusqlite::Result<()> {
    conn.execute(
        "DELETE FROM ai_settings WHERE key = ?1",
        rusqlite::params![key],
    )?;
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
    fn defaults_are_cloud_disabled_and_everything_else_empty() {
        let conn = make_conn();
        let settings = get_settings(&conn).unwrap();
        assert_eq!(settings, AiSettings::default());
    }

    #[test]
    fn settings_roundtrip() {
        let conn = make_conn();
        let settings = AiSettings {
            cloud_enabled: true,
            cloud_base_url: Some("https://api.example.com/v1".to_string()),
            cloud_model: Some("gpt-4o-mini".to_string()),
            cloud_api_key: Some("sk-test-123".to_string()),
            chat_model: Some("llama3.2:1b".to_string()),
        };
        set_settings(&conn, &settings).unwrap();
        assert_eq!(get_settings(&conn).unwrap(), settings);
    }

    #[test]
    fn setting_empty_string_clears_the_value() {
        let conn = make_conn();
        set_settings(
            &conn,
            &AiSettings {
                cloud_api_key: Some("sk-test-123".to_string()),
                ..Default::default()
            },
        )
        .unwrap();
        set_settings(
            &conn,
            &AiSettings {
                cloud_api_key: Some(String::new()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(get_settings(&conn).unwrap().cloud_api_key, None);
    }
}
