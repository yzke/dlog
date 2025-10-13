// src/main.rs

use clap::{Parser, Subcommand};
use std::io::{self, Write, Read};
use std::process::Command;
use rusqlite::Connection;
use std::env;
use chrono::{DateTime, Utc, NaiveDate};

mod db;
use db::DlogError;

// We'll use DlogError throughout
type Result<T> = std::result::Result<T, DlogError>;

#[derive(Debug)]
struct LogEntry {
    id: i32,
    timestamp: String,
    content: String,
    tags: Option<String>,
    directory: String,
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize the dlog database at ~/.config/dlog/dlog.db
    Init,

    /// Log a new entry. Use -m for short message, -t for tags.
    Log {
        #[arg(short, long, help = "Short message (like git -m)")]
        message: Option<String>,
        #[arg(short, long, help = "Comma-separated tags")]
        tags: Option<String>,
    },

    /// Get logs from current directory (or subdirs with -r)
    Get {
        #[arg(short, long, help = "Show latest N entries (default: 5)")]
        num: Option<u32>,
        #[arg(short, long, help = "Recursive: include subdirectories")]
        recursive: bool,
        #[arg(short, long, help = "Show tags in output")]
        tags: bool,
        #[arg(long, help = "Filter by date (YYYY-MM-DD)")]
        date: Option<String>,
        #[arg(short, long, help = "Search keyword in content/tags")]
        search: Option<String>,
    },

    /// Edit a log entry by ID
    Fix {
        #[arg(help = "Log ID to edit")]
        id: i32,
    },

    /// Delete a log entry by ID (with confirmation)
    Del {
        #[arg(help = "Log ID to delete")]
        id: i32,
    },
}

fn log_entry(message: Option<String>, tags: Option<String>) -> Result<()> {
    let content = if let Some(msg) = message {
        msg
    } else {
        println!("Enter your log (Ctrl+D to finish):");
        let mut input = String::new();
        io::stdin().read_to_string(&mut input)
            .map_err(|e| DlogError::Io(e))?;
        if input.trim().is_empty() {
            eprintln!("Empty log, skipped.");
            return Ok(());
        }
        input
    };

    let timestamp = Utc::now().to_rfc3339();
    let dir = env::current_dir()
        .map_err(|e| DlogError::Io(e))?
        .to_string_lossy()
        .to_string();

    let db_path = db::get_db_path();
    let conn = Connection::open(&db_path).map_err(DlogError::Sql)?;
    conn.execute(
        "INSERT INTO logs (timestamp, directory, content, tags) VALUES (?1, ?2, ?3, ?4)",
        (&timestamp, &dir, &content, tags.as_deref()),
    ).map_err(DlogError::Sql)?;
    println!("✓ Log recorded.");
    Ok(())
}

fn get_logs(
    num: Option<u32>,
    recursive: bool,
    show_tags: bool,
    date: Option<String>,
    search: Option<String>,
) -> Result<()> {
    let current_dir = env::current_dir()
        .map_err(|e| DlogError::Io(e))?
        .to_string_lossy()
        .to_string();

    let mut query = String::from("SELECT id, timestamp, content, tags, directory FROM logs WHERE ");
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if recursive {
        query.push_str("directory LIKE ? || '%' ");
        params.push(Box::new(current_dir));
    } else {
        query.push_str("directory = ? ");
        params.push(Box::new(current_dir));
    }

    if let Some(d) = &date {
        if NaiveDate::parse_from_str(d, "%Y-%m-%d").is_err() {
            return Err(DlogError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid date format. Use YYYY-MM-DD.",
            )));
        }
        query.push_str("AND date(timestamp) = ? ");
        params.push(Box::new(d.clone()));
    }

    if let Some(keyword) = &search {
        query.push_str("AND (content LIKE ? OR tags LIKE ?) ");
        params.push(Box::new(format!("%{}%", keyword)));
        params.push(Box::new(format!("%{}%", keyword)));
    }

    query.push_str("ORDER BY timestamp DESC LIMIT ?");
    let limit = num.unwrap_or(5);
    params.push(Box::new(limit as i32));

    let db_path = db::get_db_path();
    let conn = Connection::open(&db_path).map_err(DlogError::Sql)?;
    let mut stmt = conn.prepare(&query).map_err(DlogError::Sql)?;

    let logs: Vec<LogEntry> = stmt
        .query_map(rusqlite::params_from_iter(params.iter().map(|b| b.as_ref())), |row| {
            Ok(LogEntry {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                content: row.get(2)?,
                tags: row.get(3)?,
                directory: row.get(4)?,
            })
        })
        .map_err(DlogError::Sql)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(DlogError::Sql)?;

    if logs.is_empty() {
        println!("No logs found.");
        return Ok(());
    }

    for log in logs {
        let dt: DateTime<Utc> = log.timestamp.parse().unwrap_or(Utc::now());
        let formatted_time = dt.format("%Y-%m-%d %H:%M:%S").to_string();

        if show_tags {
            let tag_str = log.tags.unwrap_or_else(|| "–".to_string());
            println!("[{}] {} | Tags: {}", log.id, formatted_time, tag_str);
        } else {
            println!("[{}] {}", log.id, formatted_time);
        }
        println!("{}", log.content.trim_end());
        println!("{}", "─".repeat(40));
    }
    Ok(())
}

fn fix_log(id: i32) -> Result<()> {
    let db_path = db::get_db_path();
    let conn = Connection::open(&db_path).map_err(DlogError::Sql)?;

    let mut stmt = conn.prepare("SELECT content FROM logs WHERE id = ?").map_err(DlogError::Sql)?;
    let mut rows = stmt.query([id]).map_err(DlogError::Sql)?;
    if let Some(row) = rows.next().map_err(DlogError::Sql)? {
        let old_content: String = row.get(0).map_err(DlogError::Sql)?;
        let mut temp_file = tempfile::NamedTempFile::new()
            .map_err(|e| DlogError::Io(e.into()))?;
        temp_file.write_all(old_content.as_bytes()).map_err(DlogError::Io)?;
        temp_file.flush().map_err(DlogError::Io)?;

        let editor = env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
        let status = Command::new(editor).arg(temp_file.path()).status()
            .map_err(|e| DlogError::Io(e))?;
        if !status.success() {
            return Err(DlogError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Editor exited with error",
            )));
        }

        let new_content = std::fs::read_to_string(temp_file.path())
            .map_err(DlogError::Io)?;
        if new_content.trim() == old_content.trim() {
            println!("No changes made.");
            return Ok(());
        }

        conn.execute("UPDATE logs SET content = ? WHERE id = ?", (&new_content, id))
            .map_err(DlogError::Sql)?;
        println!("✓ Log #{} updated.", id);
        Ok(())
    } else {
        eprintln!("Log ID {} not found.", id);
        Ok(())
    }
}

fn del_log(id: i32) -> Result<()> {
    let db_path = db::get_db_path();
    let conn = Connection::open(&db_path).map_err(DlogError::Sql)?;

    let mut stmt = conn.prepare("SELECT timestamp, content FROM logs WHERE id = ?").map_err(DlogError::Sql)?;
    let mut rows = stmt.query([id]).map_err(DlogError::Sql)?;
    if let Some(row) = rows.next().map_err(DlogError::Sql)? {
        let ts: String = row.get(0).map_err(DlogError::Sql)?;
        let content: String = row.get(1).map_err(DlogError::Sql)?;
        let dt: DateTime<Utc> = ts.parse().unwrap_or(Utc::now());
        let fmt_time = dt.format("%Y-%m-%d %H:%M:%S").to_string();

        println!("You are about to delete:");
        println!("[{}] {}", id, fmt_time);
        println!("{}", content);
        print!("Confirm deletion? (y/N): ");
        io::stdout().flush().map_err(DlogError::Io)?;

        let mut input = String::new();
        io::stdin().read_line(&mut input).map_err(DlogError::Io)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Cancelled.");
            return Ok(());
        }

        conn.execute("DELETE FROM logs WHERE id = ?", [id]).map_err(DlogError::Sql)?;
        println!("✓ Log #{} deleted.", id);
        Ok(())
    } else {
        eprintln!("Log ID {} not found.", id);
        Ok(())
    }
}

fn main() {
    let cli = Cli::parse();

    let result = match &cli.command {
        Some(Commands::Init) => {
            let db_path = db::get_db_path();
            db::initialize_db(&db_path)
        }
        Some(Commands::Log { message, tags }) => {
            log_entry(message.clone(), tags.clone())
        }
        Some(Commands::Get { num, recursive, tags, date, search }) => {
            get_logs(*num, *recursive, *tags, date.clone(), search.clone())
        }
        Some(Commands::Fix { id }) => {
            fix_log(*id)
        }
        Some(Commands::Del { id }) => {
            del_log(*id)
        }
        None => {
            println!("Use 'dlog --help' for usage.");
            return;
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

