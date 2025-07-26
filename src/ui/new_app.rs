// hey claude! this entire thing could be moved and renamed right? This struct is the only thing in
// this module. Yeah just move this into the app mod
use anyhow::Result;
use crossterm::event::KeyEvent;
use ratatui::Frame;
use std::path::PathBuf;

use crate::db::DatabaseManager;
use crate::state::{StateContext, StateManager};

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

        // Initialize with either provided database or default in-memory database
        if let Some(ref db_path) = database_path {
            // Load the specified database
            let db_name = db_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("database")
                .to_string();
            let path_str = db_path.to_string_lossy().to_string();

            if let Err(e) = database_manager.add_database(db_name.clone(), path_str) {
                tracing::error!(
                    "Failed to load database from {}: {e}",
                    db_path.display()
                );
                // Fall back to default database if loading fails
                if let Err(e2) = database_manager.initialize_default_databases() {
                    tracing::error!("Failed to initialize fallback databases: {e2}");
                }
            } else {
                // Set the loaded database as current
                if let Err(e) = database_manager.set_current_database(&db_name) {
                    tracing::error!("Failed to set current database: {e}");
                } else {
                    tracing::info!(
                        "Successfully loaded database: {}",
                        db_path.display()
                    );
                }
            }
        } else {
            // Initialize with default databases (normal startup)
            if let Err(e) = database_manager.initialize_default_databases() {
                tracing::error!("Failed to initialize default databases: {e}");
            } else {
                tracing::info!("Default databases initialized successfully");
            }
        }

        // Create state context
        let mut context = StateContext::new(database_manager);

        // Set the CLI database flag and selected database if a database was provided
        if let Some(ref db_path) = database_path {
            context.set_cli_database_provided(true);

            // Set the selected database in GlobalState to match what was loaded
            let db_name = db_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("database")
                .to_string();
            context.set_selected_database(Some(db_name));
        }

        // Create state manager with all panels
        let mut state_manager = StateManager::new(context);

        // Initialize panels after creation - this will trigger the CLI auto-focus logic
        if let Err(e) = state_manager.initialize_panels() {
            tracing::error!("Failed to initialize panels: {}", e);
        }

        Self { state_manager }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        match self.state_manager.handle_event(key) {
            Ok(should_continue) => should_continue,
            Err(e) => {
                tracing::error!("Error handling key event: {e}");
                true // Continue running despite error
            }
        }
    }

    pub fn render(&mut self, f: &mut Frame) -> Result<()> {
        let area = f.area();
        self.state_manager.render(f, area)
    }

    pub fn tick(&mut self) {
        self.state_manager.tick();
    }
}

