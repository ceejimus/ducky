use anyhow::Result;
use std::path::PathBuf;

use crate::actions::{Action, ActionLogger, DatabaseType};
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
        self.state.show_success(format!("Connected to database: {name}"));
        self.state.select_database(name);
        Ok(())
    }

    /// Create a new in-memory database
    pub fn create_new_database(&mut self) -> Result<()> {
        let name = format!("new_db_{}", chrono::Utc::now().timestamp());
        
        let action = Action::CreateNewDatabase {
            database_name: name.clone(),
            database_type: DatabaseType::InMemory,
        };

        let tracker = self.action_logger.start_action(action);
        let result = self.execute_create_new_database(name);
        self.action_logger.complete_action(tracker, &result);
        result
    }

    fn execute_create_new_database(&mut self, name: String) -> Result<()> {
        self.database_manager
            .add_database(name.clone(), ":memory:".to_string())?;
        self.database_manager.set_current_database(&name)?;
        self.state.show_success(format!("Created database: {name}"));
        self.state.select_database(name);
        Ok(())
    }

    /// Disconnect from the current database
    pub fn disconnect_current_database(&mut self) -> Result<()> {
        let current_db = self
            .database_manager
            .get_current_database()
            .ok_or_else(|| anyhow::anyhow!("No database currently selected"))?;

        let db_name = current_db.to_string();

        let action = Action::DisconnectFromDatabase {
            database_name: db_name.clone(),
        };

        let tracker = self.action_logger.start_action(action);
        let result = self.execute_disconnect_current_database(db_name);
        self.action_logger.complete_action(tracker, &result);
        result
    }

    fn execute_disconnect_current_database(&mut self, db_name: String) -> Result<()> {
        if db_name == "memory" {
            return Err(anyhow::anyhow!(
                "Cannot disconnect from default memory database"
            ));
        }

        self.database_manager.remove_database(&db_name)?;
        self.state.show_info(format!("Disconnected from: {db_name}"));
        self.state.selected_database = None;
        self.state.selected_table = None;
        Ok(())
    }

    /// Select a database
    pub fn select_database(&mut self, db_name: String) -> Result<()> {
        let action = Action::SelectDatabase {
            database_name: db_name.clone(),
        };

        let tracker = self.action_logger.start_action(action);
        let result = self.execute_select_database(db_name);
        self.action_logger.complete_action(tracker, &result);
        result
    }

    fn execute_select_database(&mut self, db_name: String) -> Result<()> {
        self.database_manager
            .set_current_database(&db_name)
            .map_err(|e| anyhow::anyhow!("Error selecting database: {e}"))?;
        self.state.select_database(db_name);
        Ok(())
    }

    /// Select a table
    pub fn select_table(&mut self, table_name: String) -> Result<()> {
        let action = Action::SelectTable {
            table_name: table_name.clone(),
        };

        let tracker = self.action_logger.start_action(action);
        let result = self.execute_select_table(table_name);
        self.action_logger.complete_action(tracker, &result);
        result
    }

    fn execute_select_table(&mut self, table_name: String) -> Result<()> {
        self.state.select_table(table_name);
        Ok(())
    }

    /// Select a file for database connection
    pub fn select_file(&mut self, path: PathBuf) -> Result<()> {
        self.connect_to_database_file(path)
    }

    /// Import a file into a table
    pub fn import_file_to_table(&mut self, file_path: PathBuf, table_name: String) -> Result<()> {
        // Get current database connection
        let current_db = self.database_manager.get_current_database()
            .ok_or_else(|| anyhow::anyhow!("No database currently selected"))?
            .to_string();
        
        let connection = self.database_manager.get_current_connection()
            .ok_or_else(|| anyhow::anyhow!("No active database connection"))?;

        // Create import workflows
        let mut import_workflows = ImportWorkflows::new(connection, self.action_logger);
        
        // Import the file
        import_workflows.import_file_to_table(&file_path, &table_name)?;
        
        // Refresh the database to show the new table
        self.database_manager.refresh_database(&current_db)?;
        
        // Update state
        self.state.show_success(format!("Imported {} into table '{}'", 
                                        file_path.file_name().unwrap_or_default().to_string_lossy(), 
                                        table_name));
        
        // Ensure database is selected in state
        self.state.select_database(current_db);
        self.state.select_table(table_name);
        
        Ok(())
    }

    /// Save database to file
    pub fn save_database_to_file(&mut self, database_name: String, file_path: PathBuf) -> Result<()> {
        let action = Action::SaveDatabase {
            database_name: database_name.clone(),
            file_path: file_path.to_string_lossy().to_string(),
        };

        let tracker = self.action_logger.start_action(action);
        let result = self.execute_save_database_to_file(database_name, file_path);
        self.action_logger.complete_action(tracker, &result);
        result
    }

    fn execute_save_database_to_file(&mut self, database_name: String, file_path: PathBuf) -> Result<()> {
        let file_path_str = file_path.to_string_lossy().to_string();
        
        self.database_manager.save_database_to_file(&database_name, &file_path_str)?;
        
        self.state.show_success(format!(
            "Saved database '{}' to '{}'", 
            database_name, 
            file_path.file_name().unwrap_or_default().to_string_lossy()
        ));
        
        Ok(())
    }
}