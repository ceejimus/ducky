use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

pub mod logger;
pub use logger::ActionLogger;

/// Represents meaningful user actions that change state or perform I/O
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Action {
    // Database actions
    ConnectToDatabase { 
        database_name: String, 
        path: String 
    },
    DisconnectFromDatabase { 
        database_name: String 
    },
    CreateNewDatabase { 
        database_name: String,
        database_type: DatabaseType
    },
    SaveDatabase {
        database_name: String,
        file_path: String
    },
    SelectDatabase { 
        database_name: String 
    },
    
    // Table actions
    SelectTable { 
        table_name: String 
    },
    RefreshTables,
    
    // File operations
    SelectFile { 
        path: String 
    },
    
    // Query operations (for future use)
    ExecuteQuery { 
        query: String 
    },
    
    // Data import/export (for future use)
    ImportData { 
        source_path: String,
        format: String,
        destination_table: String
    },
    ExportData { 
        source_table: String,
        destination_path: String,
        format: String
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DatabaseType {
    InMemory,
    File(PathBuf),
}

/// Result of executing an action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    pub action: Action,
    pub timestamp: DateTime<Utc>,
    pub success: bool,
    pub message: Option<String>,
    pub error: Option<String>,
    pub duration_ms: u64,
}

impl ActionResult {
    pub fn success(action: Action, message: Option<String>, duration_ms: u64) -> Self {
        Self {
            action,
            timestamp: Utc::now(),
            success: true,
            message,
            error: None,
            duration_ms,
        }
    }
    
    pub fn failure(action: Action, error: String, duration_ms: u64) -> Self {
        Self {
            action,
            timestamp: Utc::now(),
            success: false,
            message: None,
            error: Some(error),
            duration_ms,
        }
    }
    
    pub fn log_result(&self) {
        // No console logging - handled by ActionLogger
    }
}

impl std::fmt::Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Action::ConnectToDatabase { database_name, path } => {
                write!(f, "Connect to database '{}' at '{}'", database_name, path)
            }
            Action::DisconnectFromDatabase { database_name } => {
                write!(f, "Disconnect from database '{}'", database_name)
            }
            Action::CreateNewDatabase { database_name, database_type } => {
                write!(f, "Create new database '{}' ({:?})", database_name, database_type)
            }
            Action::SaveDatabase { database_name, file_path } => {
                write!(f, "Save database '{}' to '{}'", database_name, file_path)
            }
            Action::SelectDatabase { database_name } => {
                write!(f, "Select database '{}'", database_name)
            }
            Action::SelectTable { table_name } => {
                write!(f, "Select table '{}'", table_name)
            }
            Action::RefreshTables => write!(f, "Refresh tables"),
            Action::SelectFile { path } => {
                write!(f, "Select file '{}'", path)
            }
            Action::ExecuteQuery { query } => {
                write!(f, "Execute query: {}", query)
            }
            Action::ImportData { source_path, format, destination_table } => {
                write!(f, "Import {} data from '{}' to table '{}'", format, source_path, destination_table)
            }
            Action::ExportData { source_table, destination_path, format } => {
                write!(f, "Export table '{}' to '{}' as {}", source_table, destination_path, format)
            }
        }
    }
}