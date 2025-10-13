// src/db.rs
use rusqlite::Connection;
use std::path::Path;

#[derive(Debug)]
pub enum DlogError {
    Io(std::io::Error),
    Sql(rusqlite::Error),
}

impl std::fmt::Display for DlogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DlogError::Io(e) => write!(f, "IO error: {}", e),
            DlogError::Sql(e) => write!(f, "Database error: {}", e),
        }
    }
}

impl std::error::Error for DlogError {}

pub fn initialize_db(db_path: &Path) -> Result<(), DlogError> {
    if let Some(parent) = db_path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent).map_err(DlogError::Io)?;
        }
    }

    let conn = Connection::open(db_path).map_err(DlogError::Sql)?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS logs (
            id          INTEGER PRIMARY KEY,
            timestamp   TEXT NOT NULL,
            directory   TEXT NOT NULL,
            content     TEXT NOT NULL,
            tags        TEXT
        )",
        [],
    ).map_err(DlogError::Sql)?;
    Ok(())
}

pub fn get_db_path() -> std::path::PathBuf {
    let home_dir = dirs::home_dir().expect("Could not find home directory");
    home_dir.join(".config/dlog/dlog.db")
}

