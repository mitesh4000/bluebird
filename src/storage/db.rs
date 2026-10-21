//! SQLite persistence for history, saved requests, and settings.

use crate::http::models::{AppSettings, HistoryEntry, RequestSpec, SavedRequest};
use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};
use uuid::Uuid;

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS history (
    id TEXT PRIMARY KEY,
    timestamp TEXT NOT NULL,
    method TEXT NOT NULL,
    url TEXT NOT NULL,
    status INTEGER,
    duration_ms INTEGER
);
CREATE TABLE IF NOT EXISTS saved_requests (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    spec_json TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
"#;

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let conn = Connection::open(path)
            .with_context(|| format!("open db {}", path.display()))?;
        conn.execute_batch(SCHEMA)?;
        let db = Self { conn };
        Ok(db)
    }

    pub fn open_default() -> Result<Self> {
        Self::open(&default_db_path())
    }

    #[cfg(test)]
    pub fn open_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self { conn })
    }

    pub fn add_history(
        &self,
        method: &str,
        url: &str,
        status: Option<u16>,
        duration_ms: Option<u64>,
    ) -> Result<()> {
        let id = Uuid::new_v4().to_string();
        let ts = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO history (id, timestamp, method, url, status, duration_ms) VALUES (?1,?2,?3,?4,?5,?6)",
            params![id, ts, method, url, status.map(|s| s as i64), duration_ms.map(|d| d as i64)],
        )?;
        Ok(())
    }

    pub fn list_history(&self, limit: u32) -> Result<Vec<HistoryEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, timestamp, method, url, status, duration_ms FROM history ORDER BY timestamp DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit], |row| {
            Ok(HistoryEntry {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                method: row.get(2)?,
                url: row.get(3)?,
                status: row.get::<_, Option<i64>>(4)?.map(|s| s as u16),
                duration_ms: row.get::<_, Option<i64>>(5)?.map(|d| d as u64),
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn clear_history(&self) -> Result<()> {
        self.conn.execute("DELETE FROM history", [])?;
        Ok(())
    }

    pub fn prune_history(&self, retention_days: u32) -> Result<()> {
        let cutoff = (Utc::now() - chrono::Duration::days(retention_days as i64)).to_rfc3339();
        self.conn
            .execute("DELETE FROM history WHERE timestamp < ?1", params![cutoff])?;
        Ok(())
    }

    pub fn save_request(&self, name: &str, spec: &RequestSpec) -> Result<String> {
        let id = Uuid::new_v4().to_string();
        let json = serde_json::to_string(spec)?;
        let ts = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO saved_requests (id, name, spec_json, updated_at) VALUES (?1,?2,?3,?4)",
            params![id, name, json, ts],
        )?;
        Ok(id)
    }

    pub fn list_saved(&self) -> Result<Vec<SavedRequest>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, spec_json, updated_at FROM saved_requests ORDER BY name")?;
        let rows = stmt.query_map([], |row| {
            Ok(SavedRequest {
                id: row.get(0)?,
                name: row.get(1)?,
                spec_json: row.get(2)?,
                updated_at: row.get(3)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn delete_saved(&self, id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM saved_requests WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn get_settings(&self) -> Result<AppSettings> {
        let mut s = AppSettings::default();
        let mut stmt = self.conn.prepare("SELECT key, value FROM settings")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        for r in rows {
            let (k, v) = r?;
            match k.as_str() {
                "redact_on_copy" => s.redact_on_copy = v == "true",
                "copy_mode" => s.copy_mode = v,
                "timeout_ms" => s.timeout_ms = v.parse().unwrap_or(30_000),
                "follow_redirects" => s.follow_redirects = v != "false",
                "max_redirects" => s.max_redirects = v.parse().unwrap_or(10),
                "history_retention_days" => s.history_retention_days = v.parse().unwrap_or(30),
                "warned_clipboard_secrets" => s.warned_clipboard_secrets = v == "true",
                _ => {}
            }
        }
        Ok(s)
    }

    pub fn save_settings(&self, s: &AppSettings) -> Result<()> {
        let pairs = [
            ("redact_on_copy", s.redact_on_copy.to_string()),
            ("copy_mode", s.copy_mode.clone()),
            ("timeout_ms", s.timeout_ms.to_string()),
            ("follow_redirects", s.follow_redirects.to_string()),
            ("max_redirects", s.max_redirects.to_string()),
            (
                "history_retention_days",
                s.history_retention_days.to_string(),
            ),
            (
                "warned_clipboard_secrets",
                s.warned_clipboard_secrets.to_string(),
            ),
        ];
        for (k, v) in pairs {
            self.conn.execute(
                "INSERT INTO settings (key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![k, v],
            )?;
        }
        Ok(())
    }
}

fn default_db_path() -> PathBuf {
    let base = dirs_next_data();
    base.join("bluebird").join("bluebird.db")
}

fn dirs_next_data() -> PathBuf {
    // Minimal data dir without extra dep
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home).join(".local").join("share");
    }
    if let Some(appdata) = std::env::var_os("APPDATA") {
        return PathBuf::from(appdata);
    }
    std::env::temp_dir()
}
