// src/main.rs

mod cli;
mod commands;
mod db;
mod error;
mod models;

use cli::{Cli, Commands};
use clap::Parser;
use error::Result;

fn main() {
    let cli = Cli::parse();

    // 运行命令并处理结果
    if let Err(e) = run_command(cli.command) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run_command(command: Commands) -> Result<()> {
    match command {
        Commands::Init => commands::handle_init(),
        Commands::Log { message, tags } => commands::handle_log(message, tags),
        Commands::Get { path, num, recursive, tag, date, search } => {
            commands::handle_get(path, num, recursive, tag, date, search)
        }
        Commands::Fix { id } => commands::handle_fix(id),
        Commands::Del { ids, recursive } => commands::handle_del(ids, recursive),
    }
}
