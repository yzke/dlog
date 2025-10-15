// src/cli.rs

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    author = "Your Name",
    version = "0.2.0",
    about = "dlog - A developer log tool for the command line",
    long_about = "dlog helps you keep track of your development progress by logging entries associated with specific directories. It's like a personal diary for your projects, stored locally and accessed via your terminal."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Initializes the dlog database and syncs directories.
    /// It checks for log entries pointing to non-existent directories and prompts for cleanup.
    Init,

    /// Adds a new log entry to the current directory.
    /// If no message is provided via -m, it opens the default editor.
    Log {
        #[arg(short, long, help = "A short, one-line message for the log entry")]
        message: Option<String>,
        #[arg(short, long, help = "Comma-separated tags to categorize the log")]
        tags: Option<String>,
    },

    /// Retrieves and displays log entries.
    /// By default, it shows logs from the current directory.
    Get {
        /// The directory to get logs from. Defaults to the current directory.
        path: Option<String>,

        #[arg(short, long, help = "Show latest N entries (default: 10)")]
        num: Option<u32>,

        #[arg(short, long, help = "Recursively include subdirectories in the search")]
        recursive: bool,

        #[arg(short, long, help = "Filter logs by a specific tag")]
        tag: Option<String>,

        #[arg(long, help = "Filter logs by a specific date (format: YYYY-MM-DD)")]
        date: Option<String>,

        #[arg(short, long, help = "Search for a keyword in log content and tags")]
        search: Option<String>,
    },

    /// Edits an existing log entry by its ID using the default editor.
    Fix {
        #[arg(help = "The numeric ID of the log entry to edit")]
        id: i32,
    },

    /// Deletes one or more log entries.
    #[command(verbatim_doc_comment)]
    Del {
        /// A list of log IDs to delete.
        /// Can be a single ID, comma-separated IDs, or a range.
        /// Examples:
        ///   dlog del 5          (deletes log #5)
        ///   dlog del 3,5,8      (deletes logs #3, #5, #8)
        ///   dlog del 7-9        (deletes logs #7, #8, #9)
        ///   dlog del 3,7-9,12   (deletes logs #3, #7, #8, #9, #12)
        #[arg(conflicts_with = "recursive", value_name = "ID_LIST")]
        ids: Option<String>,

        /// Recursively delete all logs in the current directory and its subdirectories.
        #[arg(short, long, help = "Recursively delete all logs from the current path downwards")]
        recursive: bool,
    },
}
