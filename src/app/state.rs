
use crate::db::query::QueryResult;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppState {
    DatabaseBrowser,
    TableViewer,
    QueryEditor,
    ImportWizard,
    ExportWizard,
    Settings,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotificationType {
    Success,
    Error,
    Info,
}

#[derive(Debug, Clone)]
pub struct Notification {
    pub message: String,
    pub notification_type: NotificationType,
    pub timestamp: std::time::Instant,
    pub duration_secs: u64,
}

impl Notification {
    pub fn success(message: String) -> Self {
        Self {
            message,
            notification_type: NotificationType::Success,
            timestamp: std::time::Instant::now(),
            duration_secs: 3,
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            message,
            notification_type: NotificationType::Error,
            timestamp: std::time::Instant::now(),
            duration_secs: 5, // Errors stay longer
        }
    }

    pub fn info(message: String) -> Self {
        Self {
            message,
            notification_type: NotificationType::Info,
            timestamp: std::time::Instant::now(),
            duration_secs: 3,
        }
    }

    pub fn is_expired(&self) -> bool {
        self.timestamp.elapsed().as_secs() >= self.duration_secs
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::DatabaseBrowser
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NavigationPanel {
    DatabaseList,
    TableList,
    MainContent,
    StatusBar,
}

impl Default for NavigationPanel {
    fn default() -> Self {
        Self::DatabaseList
    }
}

#[derive(Debug, Clone, Default)]
pub struct ApplicationState {
    pub current_state: AppState,
    pub active_panel: NavigationPanel,
    pub selected_database: Option<String>,
    pub selected_table: Option<String>,
    pub status_message: String,
    // Table creation state
    pub is_creating_table: bool,
    pub new_table_name: String,
    pub table_creation_step: TableCreationStep,
    // Notification system
    pub notifications: Vec<Notification>,
    // Table data display state
    pub table_data: Option<QueryResult>,
    pub scroll_x: usize,
    pub scroll_y: usize,
    pub selected_row: usize,
    pub selected_col: usize,
    pub page_size: usize,
    // Database dropdown state
    pub database_dropdown_expanded: bool,
    pub dropdown_selected_index: usize,
    // Navigation state - track which left panel widget was last active
    pub last_left_panel: NavigationPanel,
    // Flash effect for panel selection
    pub panel_flash_timer: Option<std::time::Instant>,
    pub flash_duration_ms: u64,
    // Delete confirmation state
    pub delete_confirmation: DeleteConfirmationState,
    // Database name input state
    pub is_entering_database_name: bool,
    pub new_database_name: String,
    // Save filename input state
    pub is_entering_save_filename: bool,
    pub save_filename: String,
    // Column expansion state - support multiple expanded columns
    pub expanded_columns: std::collections::HashSet<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum TableCreationStep {
    #[default]
    EnteringTableName,
    SelectingFile,
    ImportingData,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeleteConfirmationState {
    None,
    Database(String), // Database name to delete
    Table(String),    // Table name to delete
}

impl Default for DeleteConfirmationState {
    fn default() -> Self {
        Self::None
    }
}

impl ApplicationState {
    pub fn new() -> Self {
        Self {
            current_state: AppState::DatabaseBrowser,
            active_panel: NavigationPanel::DatabaseList,
            selected_database: None,
            selected_table: None,
            status_message: "Ready".to_string(),
            is_creating_table: false,
            new_table_name: String::new(),
            table_creation_step: TableCreationStep::default(),
            notifications: Vec::new(),
            table_data: None,
            scroll_x: 0,
            scroll_y: 0,
            selected_row: 0,
            selected_col: 0,
            page_size: 20,
            database_dropdown_expanded: false,
            dropdown_selected_index: 0,
            last_left_panel: NavigationPanel::DatabaseList,
            panel_flash_timer: None,
            flash_duration_ms: 1000, // 1.0 second
            delete_confirmation: DeleteConfirmationState::None,
            is_entering_database_name: false,
            new_database_name: String::new(),
            is_entering_save_filename: false,
            save_filename: String::new(),
            expanded_columns: std::collections::HashSet::new(),
        }
    }

    pub fn set_status(&mut self, message: String) {
        self.status_message = message;
    }

    pub fn get_status_display(&self) -> String {
        let mut parts = Vec::new();
        
        if let Some(db) = &self.selected_database {
            parts.push(format!("Database: {db}"));
        }
        
        if let Some(table) = &self.selected_table {
            parts.push(format!("Table: {table}"));
        }
        
        if self.is_creating_table {
            match self.table_creation_step {
                TableCreationStep::EnteringTableName => parts.push("Creating table...".to_string()),
                TableCreationStep::SelectingFile => parts.push("Selecting file...".to_string()),
                TableCreationStep::ImportingData => parts.push("Importing data...".to_string()),
            }
        }
        
        if parts.is_empty() {
            self.status_message.clone()
        } else {
            format!("{} | {}", parts.join(" | "), self.status_message)
        }
    }

    // Notification system methods
    pub fn add_notification(&mut self, notification: Notification) {
        self.notifications.push(notification);
    }

    pub fn show_success(&mut self, message: String) {
        self.add_notification(Notification::success(message));
    }

    pub fn show_error(&mut self, message: String) {
        self.add_notification(Notification::error(message));
    }

    pub fn show_info(&mut self, message: String) {
        self.add_notification(Notification::info(message));
    }

    pub fn remove_expired_notifications(&mut self) {
        self.notifications.retain(|n| !n.is_expired());
    }

    pub fn select_database(&mut self, database: String) {
        self.selected_database = Some(database);
    }

    pub fn select_table(&mut self, table: String) {
        self.selected_table = Some(table);
        // Clear table data when changing tables
        self.table_data = None;
        self.scroll_x = 0;
        self.scroll_y = 0;
        self.selected_row = 0;
        self.selected_col = 0;
        // Clear expanded columns when switching tables
        self.expanded_columns.clear();
    }

    pub fn next_panel(&mut self) {
        use NavigationPanel::*;
        let new_panel = match self.active_panel {
            DatabaseList | TableList => {
                // From any left panel widget, go to data viewer
                MainContent
            },
            MainContent => {
                // From data viewer, go back to last used left panel widget
                self.last_left_panel.clone()
            },
            StatusBar => DatabaseList, // Keep existing behavior for status bar
        };
        
        // Trigger flash effect if panel is changing
        if self.active_panel != new_panel {
            self.start_panel_flash();
        }
        
        self.active_panel = new_panel;
        
        // Update last_left_panel if we're moving to a left panel
        if matches!(self.active_panel, DatabaseList | TableList) {
            self.last_left_panel = self.active_panel.clone();
        }
    }

    pub fn prev_panel(&mut self) {
        use NavigationPanel::*;
        let new_panel = match self.active_panel {
            DatabaseList => StatusBar,
            TableList => DatabaseList,
            MainContent => TableList,
            StatusBar => MainContent,
        };
        
        // Trigger flash effect if panel is changing
        if self.active_panel != new_panel {
            self.start_panel_flash();
        }
        
        self.active_panel = new_panel;
    }

    // General panel setter with flash effect
    pub fn set_active_panel(&mut self, panel: NavigationPanel) {
        // Trigger flash effect if panel is changing
        if self.active_panel != panel {
            self.start_panel_flash();
        }
        self.active_panel = panel;
    }

    // Table creation workflow methods
    pub fn start_table_creation(&mut self) {
        self.is_creating_table = true;
        self.current_state = AppState::ImportWizard;
        self.new_table_name.clear();
        self.table_creation_step = TableCreationStep::EnteringTableName;
        self.set_status("Ready".to_string());
    }

    pub fn cancel_table_creation(&mut self) {
        self.is_creating_table = false;
        self.current_state = AppState::DatabaseBrowser;
        self.new_table_name.clear();
        self.table_creation_step = TableCreationStep::EnteringTableName;
        self.set_status("Ready".to_string());
        self.show_info("Table creation cancelled".to_string());
    }

    pub fn confirm_table_name(&mut self) {
        if !self.new_table_name.trim().is_empty() {
            self.table_creation_step = TableCreationStep::SelectingFile;
            self.set_status("Ready".to_string());
        }
    }

    pub fn add_char_to_table_name(&mut self, c: char) {
        if self.is_creating_table && self.table_creation_step == TableCreationStep::EnteringTableName {
            self.new_table_name.push(c);
        }
    }

    pub fn remove_char_from_table_name(&mut self) {
        if self.is_creating_table && self.table_creation_step == TableCreationStep::EnteringTableName {
            self.new_table_name.pop();
        }
    }

    pub fn set_importing_data(&mut self) {
        self.table_creation_step = TableCreationStep::ImportingData;
        self.set_status("Ready".to_string());
    }

    pub fn complete_table_creation(&mut self, success: bool) {
        let table_name = self.new_table_name.clone();
        
        if success {
            self.show_success(format!("Successfully created table '{table_name}'"));
            self.selected_table = Some(table_name);
            // Set focus to table panel so user can navigate tables
            self.active_panel = NavigationPanel::TableList;
        } else {
            self.show_error(format!("Failed to create table '{table_name}'"));
        }
        
        self.is_creating_table = false;
        self.current_state = AppState::DatabaseBrowser;
        self.new_table_name.clear();
        self.table_creation_step = TableCreationStep::EnteringTableName;
        self.set_status("Ready".to_string());
    }

    pub fn set_table_data(&mut self, data: QueryResult) {
        self.table_data = Some(data);
        self.scroll_x = 0;
        self.scroll_y = 0;
        self.selected_row = 0;
        self.selected_col = 0;
    }

    pub fn scroll_table_left(&mut self) {
        if self.scroll_x > 0 {
            self.scroll_x -= 1;
        }
    }

    pub fn scroll_table_right(&mut self, max_columns: usize, visible_columns: usize) {
        if self.scroll_x + visible_columns < max_columns {
            self.scroll_x += 1;
        }
    }

    pub fn scroll_table_up(&mut self) {
        if self.scroll_y > 0 {
            self.scroll_y -= 1;
        }
    }

    pub fn scroll_table_down(&mut self, max_rows: usize, visible_rows: usize) {
        if self.scroll_y + visible_rows < max_rows {
            self.scroll_y += 1;
        }
    }

    pub fn page_table_up(&mut self) {
        self.scroll_y = self.scroll_y.saturating_sub(self.page_size);
    }

    pub fn page_table_down(&mut self, max_rows: usize, visible_rows: usize) {
        let new_scroll = self.scroll_y + self.page_size;
        if new_scroll + visible_rows <= max_rows {
            self.scroll_y = new_scroll;
        } else if max_rows > visible_rows {
            self.scroll_y = max_rows - visible_rows;
        }
    }

    // Database dropdown methods
    pub fn expand_database_dropdown(&mut self, num_databases: usize) {
        self.database_dropdown_expanded = true;
        // Ensure dropdown_selected_index is within bounds
        if self.dropdown_selected_index >= num_databases {
            self.dropdown_selected_index = 0;
        }
    }

    pub fn collapse_database_dropdown(&mut self) {
        self.database_dropdown_expanded = false;
    }

    pub fn dropdown_move_up(&mut self) {
        if self.dropdown_selected_index > 0 {
            self.dropdown_selected_index -= 1;
        }
    }

    pub fn dropdown_move_down(&mut self, num_databases: usize) {
        if self.dropdown_selected_index < num_databases.saturating_sub(1) {
            self.dropdown_selected_index += 1;
        }
    }

    pub fn set_dropdown_to_current_database(&mut self, current_db_index: usize) {
        self.dropdown_selected_index = current_db_index;
    }

    // Left panel navigation helper
    pub fn set_left_panel(&mut self, panel: NavigationPanel) {
        // Only track changes to actual left panel widgets
        if matches!(panel, NavigationPanel::DatabaseList | NavigationPanel::TableList) {
            self.last_left_panel = panel.clone();
        }
        
        // Trigger flash effect if panel is changing
        if self.active_panel != panel {
            self.start_panel_flash();
        }
        
        self.active_panel = panel;
    }

    // Flash effect methods
    pub fn start_panel_flash(&mut self) {
        self.panel_flash_timer = Some(std::time::Instant::now());
    }

    pub fn is_panel_flashing(&self) -> bool {
        if let Some(start_time) = self.panel_flash_timer {
            start_time.elapsed().as_millis() < self.flash_duration_ms as u128
        } else {
            false
        }
    }

    pub fn update_flash_timer(&mut self) {
        if let Some(start_time) = self.panel_flash_timer {
            if start_time.elapsed().as_millis() >= self.flash_duration_ms as u128 {
                self.panel_flash_timer = None;
            }
        }
    }

    // Delete confirmation methods
    pub fn start_database_delete_confirmation(&mut self, database_name: String) {
        self.delete_confirmation = DeleteConfirmationState::Database(database_name);
    }

    pub fn start_table_delete_confirmation(&mut self, table_name: String) {
        self.delete_confirmation = DeleteConfirmationState::Table(table_name);
    }

    pub fn cancel_delete_confirmation(&mut self) {
        self.delete_confirmation = DeleteConfirmationState::None;
    }

    pub fn is_delete_confirmation_active(&self) -> bool {
        !matches!(self.delete_confirmation, DeleteConfirmationState::None)
    }

    // Database name input methods
    pub fn start_database_name_input(&mut self) {
        self.is_entering_database_name = true;
        self.new_database_name.clear();
    }

    pub fn cancel_database_name_input(&mut self) {
        self.is_entering_database_name = false;
        self.new_database_name.clear();
    }

    pub fn add_char_to_database_name(&mut self, c: char) {
        self.new_database_name.push(c);
    }

    pub fn remove_char_from_database_name(&mut self) {
        self.new_database_name.pop();
    }

    // Save filename input methods
    pub fn start_save_filename_input(&mut self) {
        self.is_entering_save_filename = true;
        self.save_filename.clear();
    }

    pub fn cancel_save_filename_input(&mut self) {
        self.is_entering_save_filename = false;
        self.save_filename.clear();
    }

    pub fn add_char_to_save_filename(&mut self, c: char) {
        self.save_filename.push(c);
    }

    pub fn remove_char_from_save_filename(&mut self) {
        self.save_filename.pop();
    }

    // Selected row/column navigation methods
    pub fn move_selected_up(&mut self) {
        if self.selected_row > 0 {
            self.selected_row -= 1;
            // Scroll up if selected row goes above visible area
            if self.selected_row < self.scroll_y {
                self.scroll_y = self.selected_row;
            }
        }
    }

    pub fn move_selected_down(&mut self, max_rows: usize, visible_rows: usize) {
        if self.selected_row + 1 < max_rows {
            self.selected_row += 1;
            // Scroll down if selected row goes below visible area
            if self.selected_row >= self.scroll_y + visible_rows {
                self.scroll_y = self.selected_row - visible_rows + 1;
            }
        }
    }

    pub fn move_selected_left(&mut self) {
        if self.selected_col > 0 {
            self.selected_col -= 1;
            // Scroll left if selected col goes left of visible area
            if self.selected_col < self.scroll_x {
                self.scroll_x = self.selected_col;
            }
        }
    }

    pub fn move_selected_right(&mut self, max_cols: usize, visible_cols: usize) {
        if self.selected_col + 1 < max_cols {
            self.selected_col += 1;
            // Scroll right if selected col goes right of visible area
            if self.selected_col >= self.scroll_x + visible_cols {
                self.scroll_x = self.selected_col - visible_cols + 1;
            }
        }
    }

    // Column expansion methods
    pub fn toggle_column_expansion(&mut self) {
        if self.expanded_columns.contains(&self.selected_col) {
            // Collapse currently expanded column
            self.expanded_columns.remove(&self.selected_col);
        } else {
            // Expand selected column
            self.expanded_columns.insert(self.selected_col);
        }
    }

    pub fn is_column_expanded(&self, column_index: usize) -> bool {
        self.expanded_columns.contains(&column_index)
    }

    pub fn clear_expanded_columns(&mut self) {
        self.expanded_columns.clear();
    }
}