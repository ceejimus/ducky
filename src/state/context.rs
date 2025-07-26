use crate::app::state::Notification;
use crate::db::DatabaseManager;
use super::GlobalState;

/// StateContext provides shared resources and data that states need to access
/// This acts as a dependency injection container for the state system
pub struct StateContext {
    pub database_manager: DatabaseManager,
    pub notifications: Vec<Notification>,
    pub global_state: GlobalState,
}

impl StateContext {
    pub fn new(
        database_manager: DatabaseManager,
    ) -> Self {
        Self {
            database_manager,
            notifications: Vec::new(),
            global_state: GlobalState::default(),
        }
    }
    
    /// Add a notification to be displayed
    pub fn add_notification(&mut self, notification: Notification) {
        self.notifications.push(notification);
    }
    
    /// Periodic tick to remove expired notifications
    pub fn tick(&mut self) {
        self.notifications.retain(|n| !n.is_expired());
    }
    
    /// Update the selected database in global state
    pub fn set_selected_database(&mut self, database: Option<String>) {
        self.global_state.selected_database = database;
    }
    
    /// Update the selected table in global state
    pub fn set_selected_table(&mut self, table: Option<String>) {
        self.global_state.selected_table = table;
    }
    
    /// Update the status message
    pub fn set_status_message(&mut self, message: String) {
        self.global_state.status_message = message;
    }
    
    /// Set the flag indicating a CLI database was provided
    pub fn set_cli_database_provided(&mut self, provided: bool) {
        self.global_state.cli_database_provided = provided;
    }
}