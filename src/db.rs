// src/db.rs

use rusqlite::{Connection, Result};
use rusqlite::ffi::ErrorCode; // <-- 添加这行来引入正确的类型
use std::path::Path;

pub fn initialize_db(db_path: &Path) -> Result<()> {
    // 检查父目录是否存在，如果不存在则创建
    if let Some(parent) = db_path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)
                .map_err(|e| rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error {
                        code: ErrorCode::CannotOpen, // <-- 这里现在是正确的
                        extended_code: 0
                    },
                    Some(format!("Failed to create directory: {}", e))
                ))?;
        }
    }

    let conn = Connection::open(db_path)?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS logs (
            id          INTEGER PRIMARY KEY,
            timestamp   TEXT NOT NULL,
            directory   TEXT NOT NULL,
            content     TEXT NOT NULL,
            tags        TEXT,
            metadata    TEXT,
            level       TEXT
        )",
        (),
    )?;

    Ok(())
}