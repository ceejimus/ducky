use anyhow::Result;
use std::fs::File;
use tracing_subscriber::{fmt, EnvFilter, prelude::*};

/// Initialize file-based logging for the TUI application
/// 
/// This configures tracing to write to a log file that is overwritten on each run.
/// Log level can be controlled via the RUST_LOG environment variable.
pub fn initialize_logging() -> Result<()> {
    // Create log file, overwriting any existing file (same behavior as before)
    let log_file = File::create("ducky.log")?;

    // Set up env filter with default level
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    // Initialize the subscriber
    tracing_subscriber::registry()
        .with(filter)
        .with(
            fmt::layer()
                .with_file(true)
                .with_line_number(true)
                .with_target(true)
                .with_writer(log_file)
        )
        .init();

    tracing::info!("Logging initialized - writing to ducky.log");
    
    Ok(())
}