// src/main.rs

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use dirs;
use std::io::{self, BufRead};
use rusqlite::Connection;
use std::env;
use chrono::{DateTime, Utc};

mod db;

// LogEntry 结构体，用于映射数据库查询结果
struct LogEntry {
    timestamp: String,
    content: String,
    tags: Option<String>,
    _directory: String,
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Initializes the dlog database. The default of dlog db file is "~/.config/dlog/dlog.db"
    Init {
        #[arg(default_value = "dlog.db")]
        db_name: String,
    },
    /// Logs a new entry. Using Ctrl+D to end log. Use `-m` to log a short intry just like `git commit -m "message"`. Use `-t "<tag>" to append a tag to this log.
    Log {
        #[arg(short, long)]
        message: Option<String>,
        #[arg(short, long)]
        tags: Option<String>,
    },
    /// Gets logs for the current directory.
    /// Use `-r` 来递归查询当前目录下的日志.
    /// Use `-t` 额外查看日志的 tag.
    /// Use `-n <Number>` 来查询最新的若干条日志. 
    Get {
        #[arg(short, long)]
        recursive: bool,
        #[arg(short, long)]
        tags: bool,
        #[arg(short, long)]
        num: Option<u32>,
    },
}

fn main() {
    let cli = Cli::parse();

    // 构建数据库文件的路径，位于 ~/.config/dlog/
    let home_dir = dirs::home_dir().expect("Could not find home directory");
    let mut config_dir = PathBuf::from(&home_dir);
    config_dir.push(".config/dlog");

    match &cli.command {
        Some(Commands::Init { db_name }) => {
            let mut db_path = config_dir.clone();
            db_path.push(db_name);

            if db_path.exists() {
                println!("Database already exists at: {}", db_path.display());
            } else {
                match db::initialize_db(&db_path) {
                    Ok(_) => println!("Database initialized at: {}", db_path.display()),
                    Err(e) => eprintln!("Error initializing database: {}", e),
                }
            }
        },
        Some(Commands::Log { message, tags }) => {
            let log_content = if let Some(msg) = message {
                // 如果用户提供了 -m 参数，直接使用其内容
                msg.clone()
            } else {
                // 否则，进入交互式输入模式
                println!("进入交互式日志记录模式。按 Ctrl+D 结束输入。");
                let mut lines = io::stdin().lock().lines();
                let mut content = String::new();
                while let Some(line) = lines.next() {
                    let line = line.expect("Failed to read line");
                    content.push_str(&line);
                    content.push('\n');
                }
                content
            };

            let log_tags = tags.clone();
            let timestamp = Utc::now().to_rfc3339();
            let directory = env::current_dir()
                .expect("Failed to get current directory")
                .to_str()
                .expect("Failed to convert path to string")
                .to_string();

            // 数据库连接和插入逻辑
            let mut db_path = config_dir.clone();
            db_path.push("dlog.db");

            match Connection::open(&db_path) {
                Ok(conn) => {
                    let mut stmt = conn.prepare(
                        "INSERT INTO logs (timestamp, directory, content, tags) VALUES (?1, ?2, ?3, ?4)",
                    ).unwrap();
                    
                    let tags_str = log_tags.unwrap_or_else(|| "".to_string());
                    
                    match stmt.execute([timestamp, directory, log_content, tags_str]) {
                        Ok(_) => println!("日志已成功记录。"),
                        Err(e) => eprintln!("记录日志时发生错误: {}", e),
                    }
                },
                Err(e) => eprintln!("无法连接到数据库: {}", e),
            }
        },
        Some(Commands::Get { recursive, tags, num }) => {
            let current_dir = std::env::current_dir()
                .expect("Failed to get current directory")
                .to_str()
                .expect("Failed to convert path to string")
                .to_string();


            let mut query = String::from("SELECT timestamp, content, tags, directory FROM logs WHERE ");
            let mut params: Vec<String> = Vec::new();

            if *recursive {
                query.push_str("directory LIKE ? || '%' ");
                params.push(current_dir);
            } else {
                query.push_str("directory = ? ");
                params.push(current_dir);
            }

            query.push_str("ORDER BY timestamp DESC");

            let num_entries = num.or(Some(1)); // <--- 关键修改：如果 num 为 None，则默认为 Some(1)
            
            if let Some(n) = num_entries {
                query.push_str(" LIMIT ?");
                params.push(n.to_string());
            }

            let params_slice: Vec<&dyn rusqlite::ToSql> = params.iter().map(|s| s as &dyn rusqlite::ToSql).collect();

            let home_dir = dirs::home_dir().expect("Could not find home directory");
            let mut db_path = PathBuf::from(&home_dir);
            db_path.push(".config/dlog/dlog.db");

            get_logs_and_print(&db_path, &query, &params_slice, *tags);
        },
        None => {
            println!("没有指定任何命令。使用 --help 查看可用命令。");
        }
    }
}

fn get_logs_and_print(db_path: &PathBuf, query: &str, params: &[&dyn rusqlite::ToSql], with_tags: bool) {
    let conn = match Connection::open(db_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("无法连接到数据库: {}", e);
            return;
        }
    };

    let mut stmt = match conn.prepare(query) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("SQL 查询准备失败: {}", e);
            return;
        }
    };

    let log_iter = match stmt.query_map(params, |row| {
        Ok(LogEntry {
            timestamp: row.get(0)?,
            content: row.get(1)?,
            tags: row.get(2)?,
            _directory: row.get(3)?,
        })
    }) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("查询执行失败: {}", e);
            return;
        }
    };

    for log in log_iter {
        let log = match log {
            Ok(l) => l,
            Err(e) => {
                eprintln!("读取日志条目时发生错误: {}", e);
                continue;
            }
        };

        // 格式化输出
        let dt: DateTime<Utc> = log.timestamp.parse().unwrap_or_else(|_| Utc::now());
        let formatted_time = dt.format("%Y-%m-%d %H:%M:%S").to_string();

        if with_tags {
            let tags_str = log.tags.unwrap_or_else(|| "No tags".to_string());
            println!("时间: {} | 标签: {}", formatted_time, tags_str);
        } else {
            println!("时间: {}", formatted_time);
        }
        
        println!("{}", log.content);
        println!("=============================");
    }
}