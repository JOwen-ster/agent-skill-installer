/// SQLite database for persisting search history and install records.

use rusqlite::{Connection, Result as SqlResult, params};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Open (or create) the SQLite database at the given path, initializing tables.
pub fn open_db(db_path: &Path) -> SqlResult<Connection> {
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let conn = Connection::open(db_path)?;
    init_tables(&conn)?;
    Ok(conn)
}

fn init_tables(conn: &Connection) -> SqlResult<()> {
    conn.execute_batch("
        PRAGMA journal_mode=WAL;

        CREATE TABLE IF NOT EXISTS searches (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            github_url TEXT NOT NULL,
            owner TEXT NOT NULL,
            repo TEXT NOT NULL,
            branch TEXT NOT NULL,
            searched_at TEXT NOT NULL,
            agent TEXT NOT NULL DEFAULT 'codex'
        );

        CREATE TABLE IF NOT EXISTS installs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            skill_name TEXT NOT NULL,
            source_type TEXT NOT NULL,
            source_ref TEXT,
            installed_path TEXT NOT NULL,
            files_written INTEGER NOT NULL,
            installed_at TEXT NOT NULL,
            status TEXT NOT NULL,
            error_message TEXT,
            agent TEXT NOT NULL DEFAULT 'codex'
        );
    ")?;

    // Migrate existing DBs silently
    let _ = conn.execute("ALTER TABLE searches ADD COLUMN agent TEXT NOT NULL DEFAULT 'codex'", []);
    let _ = conn.execute("ALTER TABLE installs ADD COLUMN agent TEXT NOT NULL DEFAULT 'codex'", []);

    Ok(())
}

/// Record a GitHub search event.
pub fn record_search(
    conn: &Connection,
    agent: &str,
    github_url: &str,
    owner: &str,
    repo: &str,
    branch: &str,
) -> SqlResult<()> {
    let now = chrono_now();
    conn.execute(
        "INSERT INTO searches (github_url, owner, repo, branch, searched_at, agent) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![github_url, owner, repo, branch, now, agent],
    )?;
    Ok(())
}

/// Record a successful install.
pub fn record_install_ok(
    conn: &Connection,
    agent: &str,
    skill_name: &str,
    source_type: &str,
    source_ref: Option<&str>,
    installed_path: &str,
    files_written: i64,
) -> SqlResult<()> {
    let now = chrono_now();
    conn.execute(
        "INSERT INTO installs (skill_name, source_type, source_ref, installed_path, files_written, installed_at, status, error_message, agent)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'ok', NULL, ?7)",
        params![skill_name, source_type, source_ref, installed_path, files_written, now, agent],
    )?;
    Ok(())
}

/// Record a failed install.
pub fn record_install_error(
    conn: &Connection,
    agent: &str,
    skill_name: &str,
    source_type: &str,
    source_ref: Option<&str>,
    error_message: &str,
) -> SqlResult<()> {
    let now = chrono_now();
    conn.execute(
        "INSERT INTO installs (skill_name, source_type, source_ref, installed_path, files_written, installed_at, status, error_message, agent)
         VALUES (?1, ?2, ?3, '', 0, ?4, 'error', ?5, ?6)",
        params![skill_name, source_type, source_ref, now, error_message, agent],
    )?;
    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchRecord {
    pub id: i64,
    pub github_url: String,
    pub owner: String,
    pub repo: String,
    pub branch: String,
    pub searched_at: String,
    pub agent: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InstallRecord {
    pub id: i64,
    pub skill_name: String,
    pub source_type: String,
    pub source_ref: Option<String>,
    pub installed_path: String,
    pub files_written: i64,
    pub installed_at: String,
    pub status: String,
    pub error_message: Option<String>,
    pub agent: String,
}

/// Fetch all search records (most recent first).
pub fn get_searches(conn: &Connection) -> SqlResult<Vec<SearchRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, github_url, owner, repo, branch, searched_at, agent FROM searches ORDER BY id DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(SearchRecord {
            id: row.get(0)?,
            github_url: row.get(1)?,
            owner: row.get(2)?,
            repo: row.get(3)?,
            branch: row.get(4)?,
            searched_at: row.get(5)?,
            agent: row.get(6)?,
        })
    })?;
    rows.collect()
}

/// Fetch all install records (most recent first).
pub fn get_installs(conn: &Connection) -> SqlResult<Vec<InstallRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, skill_name, source_type, source_ref, installed_path, files_written, installed_at, status, error_message, agent
         FROM installs ORDER BY id DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(InstallRecord {
            id: row.get(0)?,
            skill_name: row.get(1)?,
            source_type: row.get(2)?,
            source_ref: row.get(3)?,
            installed_path: row.get(4)?,
            files_written: row.get(5)?,
            installed_at: row.get(6)?,
            status: row.get(7)?,
            error_message: row.get(8)?,
            agent: row.get(9)?,
        })
    })?;
    rows.collect()
}

/// Fetch a mapping from skill_name to the source repo for successfully installed skills.
pub fn get_install_sources(conn: &Connection) -> SqlResult<std::collections::HashMap<String, String>> {
    let mut stmt = conn.prepare(
        "SELECT skill_name, source_ref FROM installs WHERE status = 'ok' AND source_type = 'github' ORDER BY id ASC",
    )?;
    let mut map = std::collections::HashMap::new();
    let rows = stmt.query_map([], |row| {
        let name: String = row.get(0)?;
        let ref_opt: Option<String> = row.get(1)?;
        Ok((name, ref_opt))
    })?;
    
    for row in rows {
        if let Ok((name, Some(source_ref))) = row {
            // source_ref is "https://github.com/owner/repo/tree/branch/..."
            if let Some(stripped) = source_ref.strip_prefix("https://github.com/") {
                let parts: Vec<&str> = stripped.split('/').collect();
                if parts.len() >= 2 {
                    map.insert(name, format!("{}/{}", parts[0], parts[1]));
                }
            }
        }
    }
    Ok(map)
}

fn chrono_now() -> String {
    // Use a simple ISO 8601 timestamp without external chrono dependency.
    // Tauri already includes std::time.
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Format as ISO 8601 UTC
    let s = secs;
    let sec = s % 60;
    let min = (s / 60) % 60;
    let hour = (s / 3600) % 24;
    let days = s / 86400;
    // Rough date calculation (good enough for a log timestamp)
    let year_from_epoch = days_to_ymd(days);
    format!(
        "{}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year_from_epoch.0, year_from_epoch.1, year_from_epoch.2, hour, min, sec
    )
}

fn days_to_ymd(days: u64) -> (u64, u64, u64) {
    // Days since 1970-01-01
    let mut year = 1970u64;
    let mut remaining = days;
    loop {
        let days_in_year = if is_leap(year) { 366 } else { 365 };
        if remaining < days_in_year {
            break;
        }
        remaining -= days_in_year;
        year += 1;
    }
    let months = [31u64, if is_leap(year) { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut month = 1u64;
    for &days_in_month in &months {
        if remaining < days_in_month {
            break;
        }
        remaining -= days_in_month;
        month += 1;
    }
    (year, month, remaining + 1)
}

fn is_leap(year: u64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}
