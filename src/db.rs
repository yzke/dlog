// src/db.rs

use crate::error::{DlogError, Result};
use crate::models::LogEntry;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::{Path, PathBuf};

/// 获取数据库文件的标准路径 (~/.config/dlog/dlog.db)
pub fn get_db_path() -> Result<PathBuf> {
    let home_dir = dirs::home_dir().ok_or(DlogError::HomeDirNotFound)?;
    Ok(home_dir.join(".config/dlog/dlog.db"))
}

/// 打开数据库连接
pub fn open_connection() -> Result<Connection> {
    let db_path = get_db_path()?;
    Connection::open(&db_path).map_err(DlogError::Sql)
}

/// 初始化数据库，如果表不存在则创建
pub fn initialize_db() -> Result<()> {
    let db_path = get_db_path()?;
    if let Some(parent) = db_path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let conn = Connection::open(&db_path)?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS logs (
            id INTEGER PRIMARY KEY,
            timestamp TEXT NOT NULL,
            directory TEXT NOT NULL,
            content TEXT NOT NULL,
            tags TEXT
        )",
        [],
    )?;
    Ok(())
}

/// 向数据库中插入一条新的日志
pub fn add_log(
    conn: &Connection,
    dir: &str,
    content: &str,
    tags: Option<&str>,
) -> Result<()> {
    // 生成 RFC3339 格式的时间戳字符串
    let timestamp = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO logs (timestamp, directory, content, tags) VALUES (?1, ?2, ?3, ?4)",
        params![timestamp, dir, content, tags],
    )?;
    Ok(())
}

/// 规范化路径，确保路径格式一致
fn normalize_path(path: &Path) -> Result<String> {
    // 将路径转换为绝对路径
    let absolute_path = if path.is_relative() {
        std::env::current_dir()?.join(path)
    } else {
        path.to_path_buf()
    };
    
    // 规范化路径：移除尾随斜杠，解析 . 和 ..
    let canonical_path = absolute_path.canonicalize().unwrap_or(absolute_path);
    
    // 转换为字符串并确保格式一致
    Ok(canonical_path.to_string_lossy().to_string())
}

/// 根据多种条件查询日志
pub fn fetch_logs(
    conn: &Connection,
    path: &Path,
    recursive: bool,
    limit: u32,
    tag: Option<&str>,
    date: Option<&str>,
    search: Option<&str>,
) -> Result<Vec<LogEntry>> {
    // 规范化路径
    let normalized_path = normalize_path(path)?;
    
    let mut query =
        String::from("SELECT id, timestamp, content, tags, directory FROM logs WHERE ");
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if recursive {
        query.push_str("directory LIKE ? || '%' ");
        params.push(Box::new(normalized_path));
    } else {
        query.push_str("directory = ? ");
        params.push(Box::new(normalized_path));
    }

    if let Some(t) = tag {
        query.push_str("AND (tags = ? OR tags LIKE ? || ',%' OR tags LIKE '%,' || ? || ',%' OR tags LIKE '%,' || ?) ");
        params.push(Box::new(t.to_string()));
        params.push(Box::new(t.to_string()));
        params.push(Box::new(t.to_string()));
        params.push(Box::new(t.to_string()));
    }

    if let Some(d) = date {
        query.push_str("AND date(timestamp) = ? ");
        params.push(Box::new(d.to_string()));
    }

    if let Some(keyword) = search {
        query.push_str("AND (content LIKE '%' || ? || '%' OR tags LIKE '%' || ? || '%') ");
        params.push(Box::new(keyword.to_string()));
        params.push(Box::new(keyword.to_string()));
    }

    query.push_str("ORDER BY timestamp DESC LIMIT ?");
    params.push(Box::new(limit as i64));

    let mut stmt = conn.prepare(&query)?;
    let logs = stmt
        .query_map(rusqlite::params_from_iter(params.iter().map(|b| b.as_ref())), |row| {
            Ok(LogEntry {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                content: row.get(2)?,
                tags: row.get(3)?,
                directory: row.get(4)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    Ok(logs)
}

/// 根据ID获取单条日志的内容
pub fn get_log_content(conn: &Connection, id: i32) -> Result<Option<String>> {
    let content = conn
        .query_row(
            "SELECT content FROM logs WHERE id = ?",
            [id],
            |row| row.get(0),
        )
        .optional()?;
    Ok(content)
}

/// 更新日志内容
pub fn update_log_content(conn: &Connection, id: i32, new_content: &str) -> Result<usize> {
    let count = conn.execute(
        "UPDATE logs SET content = ? WHERE id = ?",
        (new_content, id),
    )?;
    Ok(count)
}

/// 根据ID列表删除日志
pub fn delete_logs_by_id(conn: &Connection, ids: &[i32]) -> Result<usize> {
    if ids.is_empty() {
        return Ok(0);
    }
    
    let placeholders = vec!["?"; ids.len()].join(",");
    let query = format!("DELETE FROM logs WHERE id IN ({})", placeholders);
    
    let mut stmt = conn.prepare(&query)?;
    let count = stmt.execute(rusqlite::params_from_iter(ids))?;
    Ok(count)
}

/// 根据路径递归查找日志
pub fn find_logs_in_path(conn: &Connection, path: &Path) -> Result<Vec<LogEntry>> {
    // 规范化路径
    let normalized_path = normalize_path(path)?;
    
    let mut stmt = conn.prepare("SELECT id, timestamp, content, tags, directory FROM logs WHERE directory LIKE ? || '%'")?;
    let logs = stmt
        .query_map([&normalized_path], |row| {
            Ok(LogEntry {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                content: row.get(2)?,
                tags: row.get(3)?,
                directory: row.get(4)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(logs)
}

/// 获取数据库中所有不重复的目录
pub fn get_distinct_directories(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT DISTINCT directory FROM logs")?;
    let dirs = stmt
        .query_map([], |row| row.get(0))?
        .collect::<std::result::Result<Vec<String>, _>>()?;
    Ok(dirs)
}

/// 根据目录列表删除日志
pub fn delete_logs_by_directory(conn: &Connection, dirs: &[String]) -> Result<usize> {
    if dirs.is_empty() {
        return Ok(0);
    }
    
    let placeholders = vec!["?"; dirs.len()].join(",");
    let query = format!("DELETE FROM logs WHERE directory IN ({})", placeholders);
    
    let mut stmt = conn.prepare(&query)?;
    let count = stmt.execute(rusqlite::params_from_iter(dirs))?;
    Ok(count)
}
