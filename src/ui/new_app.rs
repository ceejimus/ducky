use anyhow::Result;
use crossterm::event::KeyEvent;
use ratatui::Frame;
use std::path::PathBuf;

use crate::actions::ActionLogger;
use crate::db::DatabaseManager;
use crate::state::{StateManager, StateContext};

/// New App implementation using the State Pattern
pub struct NewApp {
    state_manager: StateManager,
}

impl NewApp {
    pub fn new() -> Self {
        Self::new_with_database(None)
    }

    pub fn new_with_database(database_path: Option<PathBuf>) -> Self {
        let mut database_manager = DatabaseManager::new();

        // Initialize ActionLogger
        let mut action_logger = ActionLogger::new().unwrap_or_else(|e| {
            panic!("Warning: Failed to initialize action logger: {e}");
        });

        // Initialize with either provided database or default in-memory database
        if let Some(ref db_path) = database_path {
            // Load the specified database
            let db_name = db_path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("database")
                .to_string();
            let path_str = db_path.to_string_lossy().to_string();
            
            if let Err(e) = database_manager.add_database(db_name.clone(), path_str) {
                action_logger.log_error(&format!("Failed to load database from {}: {e}", db_path.display()));
                // Fall back to default database if loading fails
                if let Err(e2) = database_manager.initialize_default_databases() {
                    action_logger.log_error(&format!("Failed to initialize fallback databases: {e2}"));
                }
            } else {
                // Set the loaded database as current
                if let Err(e) = database_manager.set_current_database(&db_name) {
                    action_logger.log_error(&format!("Failed to set current database: {e}"));
                } else {
                    action_logger.log_info(&format!("Successfully loaded database: {}", db_path.display()));
                }
            }
        } else {
            // Initialize with default databases (normal startup)
            if let Err(e) = database_manager.initialize_default_databases() {
                action_logger.log_error(&format!("Failed to initialize default databases: {e}"));
            } else {
                action_logger.log_info("Default databases initialized successfully");
            }
        }

        // Create state context
        let context = StateContext::new(database_manager, action_logger);
        
        // Create state manager with all panels
        let state_manager = StateManager::new(context);

        Self {
            state_manager,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        match self.state_manager.handle_event(key) {
            Ok(should_continue) => should_continue,
            Err(e) => {
                self.state_manager.context_mut().action_logger.log_error(&format!("Error handling key event: {e}"));
                true // Continue running despite error
            }
        }
    }

    pub fn render(&self, f: &mut Frame) -> Result<()> {
        let area = f.area();
        self.state_manager.render(f, area)
    }

    pub fn update_notifications(&mut self) {
        self.state_manager.update_notifications();
    }
}