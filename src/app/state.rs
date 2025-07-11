
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
    pub page_size: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum TableCreationStep {
    #[default]
    EnteringTableName,
    SelectingFile,
    ImportingData,
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
            page_size: 20,
        }
    }

    pub fn set_status(&mut self, message: String) {
        self.status_message = message;
    }

    pub fn get_status_display(&self) -> String {
        let mut parts = Vec::new();
        
        if let Some(db) = &self.selected_database {
            parts.push(format!("Database: {}", db));
        }
        
        if let Some(table) = &self.selected_table {
            parts.push(format!("Table: {}", table));
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
        // Don't change status message - it will be displayed via get_status_display
    }

    pub fn select_table(&mut self, table: String) {
        self.selected_table = Some(table);
        // Clear table data when changing tables
        self.table_data = None;
        self.scroll_x = 0;
        self.scroll_y = 0;
        // Don't change status message - it will be displayed via get_status_display
    }

    pub fn next_panel(&mut self) {
        use NavigationPanel::*;
        self.active_panel = match self.active_panel {
            DatabaseList => TableList,
            TableList => MainContent,
            MainContent => StatusBar,
            StatusBar => DatabaseList,
        };
    }

    pub fn prev_panel(&mut self) {
        use NavigationPanel::*;
        self.active_panel = match self.active_panel {
            DatabaseList => StatusBar,
            TableList => DatabaseList,
            MainContent => TableList,
            StatusBar => MainContent,
        };
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
            self.show_success(format!("Successfully created table '{}'", table_name));
            self.selected_table = Some(table_name);
            // Set focus to table panel so user can navigate tables
            self.active_panel = NavigationPanel::TableList;
        } else {
            self.show_error(format!("Failed to create table '{}'", table_name));
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
}