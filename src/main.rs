mod app;
mod ui;
mod db;
mod actions;
mod workflows;
mod import;

use std::io;
use std::path::PathBuf;
use clap::Parser;
use tracing::{info, error};
use std::fs::OpenOptions;
use std::io::Write;
use chrono;

use db::test_connection;

#[derive(Parser)]
#[command(name = "ducky")]
#[command(about = "A high-performance Terminal User Interface (TUI) for DuckDB")]
#[command(version)]
struct Args {
    /// Path to a DuckDB database file to connect to
    #[arg(value_name = "DATABASE")]
    database: Option<PathBuf>,
    
    /// Run without the TUI interface (useful for debugging)
    #[arg(short = 'I', long = "no-interface")]
    no_interface: bool,
    
    /// Verbose logging
    #[arg(short, long)]
    verbose: bool,
}

fn main() -> io::Result<()> {
    let args = Args::parse();
    
    // Initialize logging with appropriate level
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(if args.verbose { 
            tracing::Level::DEBUG 
        } else { 
            tracing::Level::INFO 
        })
        .finish();
    
    tracing::subscriber::set_global_default(subscriber)
        .expect("Setting default subscriber failed");
    
    // Set up panic handler to log to file
    setup_panic_handler();
    
    info!("Starting Ducky - DuckDB TUI");
    
    // Handle non-interface mode
    if args.no_interface {
        return handle_no_interface_mode(args.database);
    }
    
    // Handle direct database connection in TUI mode
    if let Some(db_path) = args.database {
        return handle_tui_with_database(db_path);
    }
    
    // Run normal TUI mode
    app::run()
}

fn setup_panic_handler() {
    std::panic::set_hook(Box::new(|panic_info| {
        let backtrace = std::backtrace::Backtrace::capture();
        
        let panic_message = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "Unknown panic".to_string()
        };
        
        let location = if let Some(location) = panic_info.location() {
            format!("{}:{}:{}", location.file(), location.line(), location.column())
        } else {
            "Unknown location".to_string()
        };
        
        let panic_log = format!(
            "🚨 PANIC OCCURRED 🚨\n\
            Time: {}\n\
            Message: {}\n\
            Location: {}\n\
            \n\
            Backtrace:\n{}\n\
            \n\
            ================================\n",
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
            panic_message,
            location,
            backtrace
        );
        
        // Log to file
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open("ducky-panic.log")
        {
            let _ = writeln!(file, "{}", panic_log);
        }
        
        // Also log to stderr for immediate visibility
        eprintln!("{}", panic_log);
    }));
}

fn handle_no_interface_mode(database: Option<PathBuf>) -> io::Result<()> {
    if let Some(db_path) = database {
        println!("Testing connection to: {}", db_path.display());
        
        let path_str = db_path.to_string_lossy().to_string();
        match test_connection(&path_str) {
            Ok(_) => {
                println!("✅ Successfully connected to database");
                
                // Try to get more info about the database
                match test_database_info(&path_str) {
                    Ok(info) => {
                        println!("📊 Database Information:");
                        println!("  Path: {}", db_path.display());
                        println!("  Version: {}", info.version);
                        println!("  Tables: {}", info.table_count);
                        if info.table_count > 0 {
                            println!("  Table names: {}", info.table_names.join(", "));
                        }
                    }
                    Err(e) => {
                        error!("Failed to get database info: {}", e);
                        eprintln!("⚠️  Connected but failed to get database info: {}", e);
                    }
                }
            }
            Err(e) => {
                error!("Connection failed: {}", e);
                eprintln!("❌ Failed to connect to database: {}", e);
                
                // Print the full error chain for debugging
                let mut current_error = e.source();
                while let Some(err) = current_error {
                    eprintln!("  Caused by: {}", err);
                    current_error = err.source();
                }
                
                std::process::exit(1);
            }
        }
    } else {
        println!("No database specified. Use --help for usage information.");
        std::process::exit(1);
    }
    
    Ok(())
}

fn handle_tui_with_database(db_path: PathBuf) -> io::Result<()> {
    // Test connection first
    let path_str = db_path.to_string_lossy().to_string();
    if let Err(e) = test_connection(&path_str) {
        error!("Failed to connect to {}: {}", db_path.display(), e);
        eprintln!("❌ Failed to connect to database: {}", e);
        std::process::exit(1);
    }
    
    info!("Successfully validated connection to {}", db_path.display());
    
    // TODO: Pass the database path to the TUI app
    // For now, just run the normal app and the user can connect manually
    app::run()
}

#[derive(Debug)]
struct DatabaseInfo {
    version: String,
    table_count: usize,
    table_names: Vec<String>,
}

fn test_database_info(path: &str) -> Result<DatabaseInfo, anyhow::Error> {
    use duckdb::Connection;
    
    let conn = if path.is_empty() || path == ":memory:" {
        Connection::open_in_memory()
    } else {
        Connection::open(path)
    }?;
    
    // Get version
    let version = conn.prepare("SELECT version()")
        .and_then(|mut stmt| stmt.query_row([], |row| Ok(row.get::<_, String>(0)?)))?;
    
    // Get table list
    let mut stmt = conn.prepare(
        "SELECT table_name FROM information_schema.tables WHERE table_schema = 'main' ORDER BY table_name"
    )?;
    
    let rows = stmt.query_map([], |row| {
        Ok(row.get::<_, String>(0)?)
    })?;
    
    let mut table_names = Vec::new();
    for row in rows {
        table_names.push(row?);
    }
    
    Ok(DatabaseInfo {
        version,
        table_count: table_names.len(),
        table_names,
    })
}
