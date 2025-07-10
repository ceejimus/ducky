use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::time::Instant;
use anyhow::Result;
use serde_json;
use tracing::{info, error};

use super::{Action, ActionResult};

/// Tracks an action in progress
pub struct ActionTracker {
    pub action: Action,
    pub start_time: Instant,
}

/// Centralized logger for user actions
pub struct ActionLogger {
    log_file: Option<BufWriter<File>>,
    #[allow(dead_code)] // Future use for Phase 2+ features  
    log_path: PathBuf,
}

impl ActionLogger {
    pub fn new() -> Result<Self> {
        let log_path = Self::get_log_path()?;
        
        // Create/overwrite the log file
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&log_path)?;
        
        let log_file = Some(BufWriter::new(file));
        
        info!("Action logger initialized at: {}", log_path.display());
        
        Ok(Self {
            log_file,
            log_path,
        })
    }
    
    fn get_log_path() -> Result<PathBuf> {
        let mut log_path = std::env::current_dir()?;
        log_path.push("ducky-actions.log");
        Ok(log_path)
    }
    
    /// Start logging an action and return a tracking ID
    pub fn start_action(&mut self, action: Action) -> ActionTracker {
        let start_time = Instant::now();
        self.log_action_start(&action);
        
        ActionTracker {
            action,
            start_time,
        }
    }
    
    /// Complete action logging with the result
    pub fn complete_action<T>(&mut self, tracker: ActionTracker, result: &Result<T>) {
        let duration = tracker.start_time.elapsed();
        let duration_ms = duration.as_millis() as u64;
        
        let action_result = match result {
            Ok(_) => ActionResult::success(tracker.action, None, duration_ms),
            Err(e) => ActionResult::failure(tracker.action, e.to_string(), duration_ms),
        };
        
        self.log_action_result(&action_result);
    }
    
    /// Execute an action and log the result (for simple cases that don't need self mutation)
    #[allow(dead_code)] // Future use for simple non-borrowing actions
    pub fn execute_action<F, R>(&mut self, action: Action, func: F) -> Result<R>
    where
        F: FnOnce() -> Result<R>,
    {
        let tracker = self.start_action(action);
        let result = func();
        self.complete_action(tracker, &result);
        result
    }
    
    fn log_action_start(&mut self, action: &Action) {
        let start_msg = format!("🚀 Starting action: {}", action);
        info!("{}", start_msg);
        
        if let Some(ref mut writer) = self.log_file {
            if let Err(e) = writeln!(writer, "{}", start_msg) {
                error!("Failed to write to action log: {}", e);
            }
        }
    }
    
    fn log_action_result(&mut self, result: &ActionResult) {
        // Log to tracing
        result.log_result();
        
        // Log to file in JSON format
        if let Some(ref mut writer) = self.log_file {
            match serde_json::to_string(result) {
                Ok(json_str) => {
                    if let Err(e) = writeln!(writer, "{}", json_str) {
                        error!("Failed to write action result to log: {}", e);
                    } else {
                        // Flush to ensure it's written immediately
                        let _ = writer.flush();
                    }
                }
                Err(e) => {
                    error!("Failed to serialize action result: {}", e);
                }
            }
        }
        
        // Log human-readable summary
        let summary = if result.success {
            format!("✅ Action completed: {} ({}ms)", result.action, result.duration_ms)
        } else {
            format!("❌ Action failed: {} - {} ({}ms)", 
                   result.action, 
                   result.error.as_ref().unwrap_or(&"Unknown error".to_string()),
                   result.duration_ms)
        };
        
        if let Some(ref mut writer) = self.log_file {
            if let Err(e) = writeln!(writer, "{}", summary) {
                error!("Failed to write summary to log: {}", e);
            } else {
                let _ = writer.flush();
            }
        }
    }
    
    /// Log an informational message
    pub fn log_info(&mut self, message: &str) {
        info!("{}", message);
        
        if let Some(ref mut writer) = self.log_file {
            let log_msg = format!("ℹ️  {}", message);
            if let Err(e) = writeln!(writer, "{}", log_msg) {
                error!("Failed to write info to log: {}", e);
            } else {
                let _ = writer.flush();
            }
        }
    }
    
    /// Log an error message
    pub fn log_error(&mut self, message: &str) {
        error!("{}", message);
        
        if let Some(ref mut writer) = self.log_file {
            let log_msg = format!("❌ ERROR: {}", message);
            if let Err(e) = writeln!(writer, "{}", log_msg) {
                error!("Failed to write error to log: {}", e);
            } else {
                let _ = writer.flush();
            }
        }
    }
    
    /// Get the path to the log file
    #[allow(dead_code)] // Future use for Phase 2+ features
    pub fn log_file_path(&self) -> &PathBuf {
        &self.log_path
    }
}

impl Drop for ActionLogger {
    fn drop(&mut self) {
        if let Some(ref mut writer) = self.log_file {
            let _ = writer.flush();
        }
        info!("Action logger closed");
    }
}