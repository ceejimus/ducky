use anyhow::Result;
use std::path::PathBuf;

use crate::actions::{Action, ActionLogger};
use crate::app::state::ApplicationState;
use crate::db::DatabaseManager;
use crate::import::ImportWorkflows;

/// Database workflow operations
pub struct DatabaseWorkflows<'a> {
    pub database_manager: &'a mut DatabaseManager,
    pub action_logger: &'a mut ActionLogger,
    pub state: &'a mut ApplicationState,
}

impl<'a> DatabaseWorkflows<'a> {
    pub fn new(
        database_manager: &'a mut DatabaseManager,
        action_logger: &'a mut ActionLogger,
        state: &'a mut ApplicationState,
    ) -> Self {
        Self {
            database_manager,
            action_logger,
            state,
        }
    }

    /// Connect to a database file
    pub fn connect_to_database_file(&mut self, path: PathBuf) -> Result<()> {
        let path_str = path.to_string_lossy().to_string();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let action = Action::ConnectToDatabase {
            database_name: name.clone(),
            path: path_str.clone(),
        };

        let tracker = self.action_logger.start_action(action);
        let result = self.execute_connect_to_database_file(name, path_str);
        self.action_logger.complete_action(tracker, &result);
        result
    }

    fn execute_connect_to_database_file(&mut self, name: String, path_str: String) -> Result<()> {
        self.database_manager.add_database(name.clone(), path_str)?;
        self.database_manager.set_current_database(&name)?;
        self.state
            .show_success(format!("Connected to database: {name}"));
        self.state.select_database(name);
        Ok(())
    }

    /// Select a file for database connection
    pub fn select_file(&mut self, path: PathBuf) -> Result<()> {
        self.connect_to_database_file(path)
    }

    /// Import a file into a table
    pub fn import_file_to_table(&mut self, file_path: PathBuf, table_name: String) -> Result<()> {
        // Get current database connection
        let current_db = self
            .database_manager
            .get_current_database()
            .ok_or_else(|| anyhow::anyhow!("No database currently selected"))?
            .to_string();

        let connection = self
            .database_manager
            .get_current_connection()
            .ok_or_else(|| anyhow::anyhow!("No active database connection"))?;

        // Create import workflows
        let mut import_workflows = ImportWorkflows::new(connection, self.action_logger);

        // Import the file
        import_workflows.import_file_to_table(&file_path, &table_name)?;

        // Refresh the database to show the new table
        self.database_manager.refresh_database(&current_db)?;

        // Update state
        self.state.show_success(format!(
            "Imported {} into table '{}'",
            file_path.file_name().unwrap_or_default().to_string_lossy(),
            table_name
        ));

        // Ensure database is selected in state
        self.state.select_database(current_db);
        self.state.select_table(table_name);

        Ok(())
    }
}

