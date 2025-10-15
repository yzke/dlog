// src/commands.rs

use crate::db;
use crate::error::{DlogError, Result};
use chrono::{DateTime, NaiveDate, Utc};
use std::collections::BTreeSet;
use std::env;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

/// 处理 'init' 命令
pub fn handle_init() -> Result<()> {
    db::initialize_db()?;
    println!("✓ Database initialized successfully at: {:?}", db::get_db_path()?);

    // 检查并同步目录
    let conn = db::open_connection()?;
    let dirs_in_db = db::get_distinct_directories(&conn)?;
    let mut deleted_dirs = Vec::new();

    for dir_str in &dirs_in_db {
        if !Path::new(dir_str).exists() {
            deleted_dirs.push(dir_str.clone());
        }
    }

    if !deleted_dirs.is_empty() {
        println!("\nWarning: The following directories with logs no longer exist:");
        for dir in &deleted_dirs {
            println!("- {}", dir);
        }
        print!("Do you want to permanently delete all logs from these directories? (y/N): ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if input.trim().eq_ignore_ascii_case("y") {
            let count = db::delete_logs_by_directory(&conn, &deleted_dirs)?;
            println!("✓ Deleted {} log entries from vanished directories.", count);
        } else {
            println!("Cancelled. No logs were deleted.");
        }
    } else {
        println!("✓ All log directories are in sync with the filesystem.");
    }

    Ok(())
}

/// 处理 'log' 命令
pub fn handle_log(message: Option<String>, tags: Option<String>) -> Result<()> {
    let content = if let Some(msg) = message {
        msg
    } else {
        // 在这个函数中 temp_file 不需要 mut，因为我们没有直接写入它
        let temp_file = tempfile::NamedTempFile::new()?;
        let editor = env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
        let status = Command::new(&editor).arg(temp_file.path()).status()?;

        if !status.success() {
            return Err(DlogError::EditorError);
        }
        let mut buf = String::new();
        temp_file.reopen()?.read_to_string(&mut buf)?;
        buf
    };

    if content.trim().is_empty() {
        eprintln!("Empty log, skipped.");
        return Ok(());
    }

    let dir = env::current_dir()?.to_string_lossy().to_string();
    let conn = db::open_connection()?;
    db::add_log(&conn, &dir, &content, tags.as_deref())?;

    println!("✓ Log recorded.");
    Ok(())
}

/// 处理 'get' 命令
pub fn handle_get(
    path: Option<String>,
    num: Option<u32>,
    recursive: bool,
    tag: Option<String>,
    date: Option<String>,
    search: Option<String>,
) -> Result<()> {
    let target_path = match path {
        Some(p) => PathBuf::from(p),
        None => env::current_dir()?,
    };

    if let Some(d) = &date {
        if NaiveDate::parse_from_str(d, "%Y-%m-%d").is_err() {
            return Err(DlogError::InvalidInput(
                "Invalid date format. Use YYYY-MM-DD.".to_string(),
            ));
        }
    }

    let limit = num.unwrap_or(10);
    let conn = db::open_connection()?;
    let logs = db::fetch_logs(
        &conn,
        &target_path,
        recursive,
        limit,
        tag.as_deref(),
        date.as_deref(),
        search.as_deref(),
    )?;

    if logs.is_empty() {
        println!("No logs found.");
        return Ok(());
    }

    for log in logs {
        // 在这里将字符串解析为 DateTime 进行格式化
        let dt: DateTime<Utc> = log.timestamp.parse().unwrap_or(Utc::now());
        let formatted_time = dt.format("%Y-%m-%d %H:%M:%S").to_string();
        let tags_display = log.tags.map_or("".to_string(), |t| format!(" | Tags: {}", t));

        println!(
            "[{}] {} {}",
            log.id,
            formatted_time,
            tags_display
        );
        // 如果是递归查询，显示日志所在目录
        if recursive {
            println!("  └─ Path: {}", log.directory);
        }
        println!("{}", log.content.trim_end());
        println!("{}", "─".repeat(40));
    }
    Ok(())
}

/// 处理 'fix' 命令
pub fn handle_fix(id: i32) -> Result<()> {
    let conn = db::open_connection()?;
    let old_content = db::get_log_content(&conn, id)?.ok_or(DlogError::LogNotFound(id))?;

    // 修正：重新添加 mut，因为我们需要调用 .write_all() 和 .flush()
    let mut temp_file = tempfile::NamedTempFile::new()?;
    temp_file.write_all(old_content.as_bytes())?;
    temp_file.flush()?;

    let editor = env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
    let status = Command::new(editor).arg(temp_file.path()).status()?;

    if !status.success() {
        return Err(DlogError::EditorError);
    }

    let new_content = std::fs::read_to_string(temp_file.path())?;
    if new_content.trim() == old_content.trim() {
        return Err(DlogError::NoChangesMade);
    }

    db::update_log_content(&conn, id, &new_content)?;
    println!("✓ Log #{} updated.", id);
    Ok(())
}

/// 解析ID范围字符串 (例如 "1,3,5-7")
fn parse_id_range(s: &str) -> Result<Vec<i32>> {
    let mut ids = BTreeSet::new(); // 使用 BTreeSet 自动排序和去重
    for part in s.split(',') {
        let part = part.trim();
        if part.contains('-') {
            let mut range_parts = part.splitn(2, '-');
            let start_str = range_parts.next().unwrap_or("").trim();
            let end_str = range_parts.next().unwrap_or("").trim();

            if start_str.is_empty() || end_str.is_empty() {
                return Err(DlogError::InvalidInput(format!("Invalid range: {}", part)));
            }
            let start: i32 = start_str.parse().map_err(|_| DlogError::InvalidInput(format!("Invalid ID: {}", start_str)))?;
            let end: i32 = end_str.parse().map_err(|_| DlogError::InvalidInput(format!("Invalid ID: {}", end_str)))?;

            if start > end {
                return Err(DlogError::InvalidInput(format!("Start of range {} cannot be greater than end {}", start, end)));
            }
            for i in start..=end {
                ids.insert(i);
            }
        } else if !part.is_empty() {
            let id: i32 = part.parse().map_err(|_| DlogError::InvalidInput(format!("Invalid ID: {}", part)))?;
            ids.insert(id);
        }
    }
    Ok(ids.into_iter().collect())
}

/// 处理 'del' 命令
pub fn handle_del(ids_str: Option<String>, recursive: bool) -> Result<()> {
    let conn = db::open_connection()?;
    let ids_to_delete = if recursive {
        let current_dir = env::current_dir()?;
        println!("Searching for logs to delete recursively from: {}", current_dir.display());
        let logs = db::find_logs_in_path(&conn, &current_dir)?;
        if logs.is_empty() {
            println!("No logs found in this directory or subdirectories.");
            return Ok(());
        }
        println!("Found {} logs to delete:", logs.len());
        for log in &logs {
            // 在这里将字符串解析为 DateTime 进行格式化
            let dt: DateTime<Utc> = log.timestamp.parse().unwrap_or(Utc::now());
            println!("- ID: {}, Date: {}", log.id, dt.format("%Y-%m-%d"));
        }
        logs.iter().map(|l| l.id).collect()
    } else if let Some(s) = ids_str {
        parse_id_range(&s)?
    } else {
        // clap应该已经阻止了这种情况，但为了安全起见
        return Err(DlogError::InvalidInput("You must provide log IDs or use the --recursive flag.".to_string()));
    };

    if ids_to_delete.is_empty() {
        println!("No valid log IDs to delete.");
        return Ok(());
    }

    println!(
        "\nYou are about to permanently delete the following log IDs: {:?}",
        ids_to_delete
    );
    print!("Confirm deletion? (y/N): ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    if !input.trim().eq_ignore_ascii_case("y") {
        println!("Cancelled.");
        return Ok(());
    }

    let count = db::delete_logs_by_id(&conn, &ids_to_delete)?;
    println!("✓ Successfully deleted {} log(s).", count);

    Ok(())
}
