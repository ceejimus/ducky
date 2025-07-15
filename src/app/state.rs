
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SortDirection {
    Ascending,
    Descending,
}

impl Default for SortDirection {
    fn default() -> Self {
        Self::Ascending
    }
}

#[derive(Debug, Clone)]
pub struct SortColumnSpec {
    pub column_name: String,
    pub direction: SortDirection,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InspectSection {
    Schema,
    Statistics,
}

impl Default for InspectSection {
    fn default() -> Self {
        Self::Schema
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
    pub selected_column: Option<String>,
    pub page_size: usize,
    // Cache last table area height for navigation calculations
    pub last_table_area_height: u16,
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
    // Save view input state
    pub is_entering_view_name: bool,
    pub new_view_name: String,
    // Column expansion state - support multiple expanded columns
    pub expanded_columns: std::collections::HashSet<usize>,
    // Multi-column sorting state
    pub sort_columns: Vec<SortColumnSpec>,
    // Search/filter state
    pub is_searching: bool,
    pub search_column: Option<usize>,
    pub search_text: String,
    pub search_syntax_valid: bool,
    pub search_debounce_timer: Option<std::time::Instant>,
    pub column_filters: std::collections::HashMap<String, String>, // column_name -> filter_text
    // Inspect mode state
    pub inspect_mode: bool,
    pub inspect_active_section: InspectSection,
    pub inspect_schema_scroll_y: usize,
    pub inspect_stats_scroll_y: usize,
    pub inspect_selected_row: usize, // Selected row in the columns view
    // Column ordering state
    pub column_order: std::collections::HashMap<String, Vec<String>>, // table_name -> ordered_column_names
    pub original_column_names: Vec<String>, // cached for current table
    // Modal modification state (reordering + hiding)
    pub is_modifying: bool,
    pub modify_backup_column_order: Option<Vec<String>>, // backup for cancel operation
    // Column hiding state
    pub hidden_columns: std::collections::HashMap<String, std::collections::HashSet<String>>, // table_name -> hidden_column_names
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
            selected_column: None,
            page_size: 20,
            last_table_area_height: 20, // Default
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
            is_entering_view_name: false,
            new_view_name: String::new(),
            expanded_columns: std::collections::HashSet::new(),
            sort_columns: Vec::new(),
            is_searching: false,
            search_column: None,
            search_text: String::new(),
            search_syntax_valid: true,
            search_debounce_timer: None,
            column_filters: std::collections::HashMap::new(),
            inspect_mode: false,
            inspect_active_section: InspectSection::Schema,
            inspect_schema_scroll_y: 0,
            inspect_stats_scroll_y: 0,
            inspect_selected_row: 0,
            column_order: std::collections::HashMap::new(),
            original_column_names: Vec::new(),
            is_modifying: false,
            modify_backup_column_order: None,
            hidden_columns: std::collections::HashMap::new(),
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
        
        if self.is_modifying {
            if self.inspect_mode {
                parts.push("MODIFY MODE: j/k to move, J/K for extremes, o to hide/show, Enter to confirm, Esc to cancel".to_string());
            } else {
                parts.push("MODIFY MODE: h/l to move, H/L for extremes, o to hide/show, Enter to confirm, Esc to cancel".to_string());
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
        self.selected_column = None;
        // Clear expanded columns when switching tables
        self.expanded_columns.clear();
        // Clear sort state when switching tables
        self.clear_sort();
        // Reset original column order cache when switching tables
        self.original_column_names.clear();
        // Note: We don't clear hidden columns when switching tables - they persist per table
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
        self.selected_column = None;
    }

    pub fn update_table_data_preserve_column(&mut self, data: QueryResult) {
        // Preserve selected column and horizontal scroll position when updating table data (used during sorting)
        let saved_scroll_x = self.scroll_x;
        
        self.table_data = Some(data);
        self.scroll_y = 0;  // Reset vertical scroll to show sorted results from top
        self.selected_row = 0;  // Reset to first row of sorted data
        
        // Note: selected_column is now name-based and doesn't need bounds checking
        self.scroll_x = saved_scroll_x;
        self.ensure_selected_column_visible();
    }

    // Helper method to ensure the selected column is visible in the current view
    fn ensure_selected_column_visible(&mut self) {
        // With name-based selection, we need to ensure the selected column index is visible
        if let Some(selected_idx) = self.get_selected_column_index() {
            if selected_idx < self.scroll_x {
                // Selected column is to the left of current view - scroll left to show it
                self.scroll_x = selected_idx;
            }
        }
        // Note: We can't easily calculate right-edge visibility here without knowing
        // the viewport width, but the existing logic should handle that
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

    // View name input methods
    pub fn start_view_name_input(&mut self) {
        self.is_entering_view_name = true;
        self.new_view_name.clear();
    }
    
    pub fn cancel_view_name_input(&mut self) {
        self.is_entering_view_name = false;
        self.new_view_name.clear();
    }
    
    pub fn add_char_to_view_name(&mut self, c: char) {
        self.new_view_name.push(c);
    }
    
    pub fn remove_char_from_view_name(&mut self) {
        self.new_view_name.pop();
    }

    // Generate SQL query for current view state
    pub fn generate_view_sql(&self, table_name: &str) -> Option<String> {
        if let Some(_table) = &self.selected_table {
            // Get visible columns in virtual order
            let visible_column_names = self.get_visible_column_names();
            
            if visible_column_names.is_empty() {
                return None;
            }
            
            // Build SELECT clause with virtual column order
            let columns_sql = visible_column_names.join(", ");
            let mut sql = format!("SELECT {columns_sql} FROM {table_name}");
            
            // Add WHERE clause for filters
            let original_column_names = self.get_original_column_names();
            if let Some(filter_clause) = self.get_filter_sql_clause(&original_column_names) {
                sql.push(' ');
                sql.push_str(&filter_clause);
            }
            
            // Add ORDER BY clause for sorting
            if let Some(sort_clause) = self.get_sort_sql_clause(&original_column_names) {
                sql.push(' ');
                sql.push_str(&sort_clause);
            }
            
            Some(sql)
        } else {
            None
        }
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
        if let Some(ref current_col) = self.selected_column.clone() {
            if let Some(prev_col) = self.get_prev_visible_column(current_col) {
                self.selected_column = Some(prev_col);
                // Ensure new selection is visible
                self.ensure_selected_column_visible();
            }
        } else {
            // No selection, select first visible column
            self.selected_column = self.get_first_visible_column();
        }
    }

    pub fn move_selected_right(&mut self, _max_cols: usize, _visible_cols: usize) {
        if let Some(ref current_col) = self.selected_column.clone() {
            if let Some(next_col) = self.get_next_visible_column(current_col) {
                self.selected_column = Some(next_col);
                // Ensure new selection is visible
                self.ensure_selected_column_visible();
            }
        } else {
            // No selection, select first visible column
            self.selected_column = self.get_first_visible_column();
        }
    }

    // Column expansion methods
    pub fn toggle_column_expansion(&mut self) {
        if let Some(selected_idx) = self.get_selected_column_index() {
            if self.expanded_columns.contains(&selected_idx) {
                // Collapse currently expanded column
                self.expanded_columns.remove(&selected_idx);
            } else {
                // Expand selected column
                self.expanded_columns.insert(selected_idx);
            }
        }
    }

    pub fn is_column_expanded(&self, column_index: usize) -> bool {
        self.expanded_columns.contains(&column_index)
    }

    pub fn clear_expanded_columns(&mut self) {
        self.expanded_columns.clear();
    }

    // Column sorting methods (temporarily disabled during refactoring)
    pub fn set_primary_sort(&mut self, ascending: bool) {
        if let Some(ref column_name) = self.selected_column {
            // Check if we're setting the same sort that already exists as primary (first in chain)
            if let Some(first_sort) = self.sort_columns.first() {
                if first_sort.column_name == *column_name {
                    let same_direction = match (&first_sort.direction, ascending) {
                        (SortDirection::Ascending, true) => true,
                        (SortDirection::Descending, false) => true,
                        _ => false,
                    };
                    
                    if same_direction {
                        // Same column and direction - clear all sorting
                        self.clear_sort();
                        return;
                    }
                }
            }
            
            // Set new primary sort (clears all existing sorts)
            self.sort_columns.clear();
            self.sort_columns.push(SortColumnSpec {
                column_name: column_name.clone(),
                direction: if ascending { SortDirection::Ascending } else { SortDirection::Descending },
            });
        }
    }

    // Toggle column in multi-column sort chain
    pub fn toggle_in_sort_chain(&mut self, ascending: bool) {
        if let Some(ref column_name) = self.selected_column {
            let desired_direction = if ascending { SortDirection::Ascending } else { SortDirection::Descending };
            
            // Check if column is already in sort chain
            if let Some(pos) = self.sort_columns.iter().position(|spec| spec.column_name == *column_name) {
                let current_spec = &self.sort_columns[pos];
                
                if current_spec.direction == desired_direction {
                    // Same direction - remove column from chain
                    self.sort_columns.remove(pos);
                } else {
                    // Different direction - update direction, keep position in chain
                    self.sort_columns[pos].direction = desired_direction;
                }
            } else {
                // Column doesn't exist - add it to end of chain
                self.sort_columns.push(SortColumnSpec {
                    column_name: column_name.clone(),
                    direction: desired_direction,
                });
            }
        }
    }

    // Helper: check if column is in sort chain
    pub fn is_column_in_sort_chain(&self, column_name: &str) -> bool {
        self.sort_columns.iter().any(|spec| spec.column_name == column_name)
    }

    pub fn clear_sort(&mut self) {
        self.sort_columns.clear();
    }

    pub fn get_sort_sql_clause(&self, _column_names: &[String]) -> Option<String> {
        if self.sort_columns.is_empty() {
            return None;
        }

        let mut sort_parts = Vec::new();
        for sort_spec in &self.sort_columns {
            let direction = match sort_spec.direction {
                SortDirection::Ascending => "ASC",
                SortDirection::Descending => "DESC",
            };
            sort_parts.push(format!("{} {}", sort_spec.column_name, direction));
        }

        if sort_parts.is_empty() {
            None
        } else {
            Some(format!("ORDER BY {}", sort_parts.join(", ")))
        }
    }

    // Search/filter methods
    pub fn start_column_search(&mut self, column_index: usize) {
        self.is_searching = true;
        self.search_column = Some(column_index);
        self.search_text.clear();
        self.search_syntax_valid = true;
        self.search_debounce_timer = None;
        
        // Auto-expand the column being searched
        self.expanded_columns.insert(column_index);
    }

    pub fn cancel_search(&mut self) {
        self.is_searching = false;
        self.search_column = None;
        self.search_text.clear();
        self.search_syntax_valid = true;
        self.search_debounce_timer = None;
    }

    pub fn add_char_to_search(&mut self, c: char) {
        if self.is_searching {
            self.search_text.push(c);
            // Reset debounce timer on new input
            self.search_debounce_timer = Some(std::time::Instant::now());
        }
    }

    pub fn remove_char_from_search(&mut self) {
        if self.is_searching {
            self.search_text.pop();
            // Reset debounce timer on input change
            self.search_debounce_timer = Some(std::time::Instant::now());
        }
    }

    pub fn finalize_search(&mut self) -> bool {
        if self.is_searching && self.search_syntax_valid && !self.search_text.trim().is_empty() {
            if let Some(column_index) = self.search_column {
                if let Some(column_name) = self.get_column_name_by_index(column_index) {
                    // Store the filter by column name
                    self.column_filters.insert(column_name, self.search_text.trim().to_string());
                    self.cancel_search();
                    return true;
                }
            }
        }
        false
    }

    pub fn is_column_filtered(&self, column_name: &str) -> bool {
        self.column_filters.contains_key(column_name)
    }

    pub fn clear_column_filter(&mut self, column_name: &str) {
        self.column_filters.remove(column_name);
    }

    pub fn clear_all_filters(&mut self) {
        self.column_filters.clear();
    }

    pub fn get_filter_sql_clause(&self, _column_names: &[String]) -> Option<String> {
        if self.column_filters.is_empty() {
            return None;
        }

        let mut filter_parts = Vec::new();
        for (column_name, filter_text) in &self.column_filters {
            // Use the filter text directly as SQL (user responsibility for syntax)
            filter_parts.push(format!("{column_name} {filter_text}"));
        }

        if filter_parts.is_empty() {
            None
        } else {
            Some(format!("WHERE {}", filter_parts.join(" AND ")))
        }
    }

    pub fn should_debounce_update(&self) -> bool {
        if let Some(timer) = self.search_debounce_timer {
            timer.elapsed().as_millis() >= 600 // 600ms debounce
        } else {
            false
        }
    }

    // Inspect mode methods
    pub fn enter_inspect_mode(&mut self) {
        self.inspect_mode = true;
        self.inspect_active_section = InspectSection::Schema;
        self.inspect_schema_scroll_y = 0;
        self.inspect_stats_scroll_y = 0;
        self.inspect_selected_row = 0;
    }

    pub fn exit_inspect_mode(&mut self) {
        self.inspect_mode = false;
        self.inspect_active_section = InspectSection::Schema;
        self.inspect_schema_scroll_y = 0;
        self.inspect_stats_scroll_y = 0;
        self.inspect_selected_row = 0;
    }

    pub fn inspect_cycle_section(&mut self) {
        self.inspect_active_section = match self.inspect_active_section {
            InspectSection::Schema => InspectSection::Statistics,
            InspectSection::Statistics => InspectSection::Schema,
        };
    }

    pub fn inspect_scroll_up(&mut self) {
        match self.inspect_active_section {
            InspectSection::Schema => {
                if self.inspect_schema_scroll_y > 0 {
                    self.inspect_schema_scroll_y -= 1;
                }
            }
            InspectSection::Statistics => {
                if self.inspect_stats_scroll_y > 0 {
                    self.inspect_stats_scroll_y -= 1;
                }
            }
        }
    }

    pub fn inspect_scroll_down(&mut self, max_rows: usize, visible_rows: usize) {
        match self.inspect_active_section {
            InspectSection::Schema => {
                if self.inspect_schema_scroll_y + visible_rows < max_rows {
                    self.inspect_schema_scroll_y += 1;
                }
            }
            InspectSection::Statistics => {
                if self.inspect_stats_scroll_y + visible_rows < max_rows {
                    self.inspect_stats_scroll_y += 1;
                }
            }
        }
    }

    pub fn inspect_move_selection_up(&mut self) {
        if self.inspect_selected_row > 0 {
            self.inspect_selected_row -= 1;
            // Auto-scroll if selection goes above visible area
            if self.inspect_selected_row < self.inspect_schema_scroll_y {
                self.inspect_schema_scroll_y = self.inspect_selected_row;
            }
        }
    }

    pub fn inspect_move_selection_down(&mut self, max_rows: usize, visible_rows: usize) {
        if self.inspect_selected_row + 1 < max_rows {
            self.inspect_selected_row += 1;
            // Auto-scroll if selection goes below visible area
            if self.inspect_selected_row >= self.inspect_schema_scroll_y + visible_rows {
                self.inspect_schema_scroll_y = self.inspect_selected_row - visible_rows + 1;
            }
        }
    }

    // Virtual column reordering methods (refactored to use names)
    pub fn initialize_column_order(&mut self, column_names: Vec<String>) {
        // Initialize original column names if not already set
        if self.original_column_names.is_empty() && !column_names.is_empty() {
            self.original_column_names = column_names.clone();
        }
        
        // Initialize virtual column order for current table if not already set
        if let Some(table_name) = &self.selected_table {
            if !self.column_order.contains_key(table_name) {
                self.column_order.insert(table_name.clone(), column_names);
            }
        }
    }

    pub fn get_virtual_column_order(&self) -> Vec<String> {
        if let Some(table_name) = &self.selected_table {
            // Return custom order if exists, otherwise return original order
            self.column_order.get(table_name)
                .cloned()
                .unwrap_or_else(|| self.original_column_names.clone())
        } else {
            self.original_column_names.clone()
        }
    }

    // Temporarily disabled during refactoring - these will be implemented with name-based logic
    pub fn reorder_column(&mut self, from_index: usize, to_index: usize) -> bool {
        if let Some(table_name) = &self.selected_table {
            let mut virtual_order = self.get_virtual_column_order();
            
            // Validate indices
            if from_index >= virtual_order.len() || to_index >= virtual_order.len() {
                return false;
            }
            
            // Perform the reorder
            let moved_column = virtual_order.remove(from_index);
            virtual_order.insert(to_index, moved_column);
            
            // Update the column order
            self.column_order.insert(table_name.clone(), virtual_order);
            
            true
        } else {
            false
        }
    }
    
    pub fn has_hidden_columns(&self) -> bool {
        if let Some(table_name) = &self.selected_table {
            self.hidden_columns
                .get(table_name)
                .is_some_and(|hidden_set| !hidden_set.is_empty())
        } else {
            false
        }
    }

    pub fn reset_column_order(&mut self) {
        if let Some(table_name) = &self.selected_table {
            self.column_order.remove(table_name);
        }
    }

    pub fn virtual_to_physical_index(&self, virtual_index: usize) -> usize {
        // TODO: Implement with name-based lookup
        virtual_index
    }

    pub fn physical_to_virtual_index(&self, physical_index: usize) -> usize {
        // TODO: Implement with name-based lookup
        physical_index
    }

    // Modal modification methods (reordering + hiding)
    pub fn start_modifying(&mut self) {
        if !self.is_modifying {
            // Backup current column order for cancel operation
            self.modify_backup_column_order = Some(self.get_virtual_column_order());
            self.is_modifying = true;
        }
    }

    pub fn cancel_modifying(&mut self) {
        if self.is_modifying {
            // Restore backup column order
            if let Some(backup_order) = &self.modify_backup_column_order {
                if let Some(table_name) = &self.selected_table {
                    self.column_order.insert(table_name.clone(), backup_order.clone());
                }
            }
            self.is_modifying = false;
            self.modify_backup_column_order = None;
        }
    }

    pub fn confirm_modifying(&mut self) {
        if self.is_modifying {
            // Just exit modifying mode - changes are already applied
            self.is_modifying = false;
            self.modify_backup_column_order = None;
        }
    }

    pub fn move_column_extreme_left(&mut self) -> bool {
        if let Some(selected_column) = &self.selected_column {
            if let Some(from_index) = self.get_column_index_by_name(selected_column) {
                self.reorder_column(from_index, 0)
            } else {
                false
            }
        } else {
            false
        }
    }

    pub fn move_column_extreme_right(&mut self) -> bool {
        if let Some(selected_column) = &self.selected_column {
            if let Some(from_index) = self.get_column_index_by_name(selected_column) {
                let virtual_order = self.get_virtual_column_order();
                if !virtual_order.is_empty() {
                    self.reorder_column(from_index, virtual_order.len() - 1)
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        }
    }

    pub fn move_column_extreme_up(&mut self) -> bool {
        if let Some(selected_column) = &self.selected_column {
            if let Some(from_index) = self.get_column_index_by_name(selected_column) {
                self.reorder_column(from_index, 0)
            } else {
                false
            }
        } else {
            false
        }
    }

    pub fn move_column_extreme_down(&mut self) -> bool {
        if let Some(selected_column) = &self.selected_column {
            if let Some(from_index) = self.get_column_index_by_name(selected_column) {
                let virtual_order = self.get_virtual_column_order();
                if !virtual_order.is_empty() {
                    self.reorder_column(from_index, virtual_order.len() - 1)
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        }
    }

    // Extreme navigation methods (when not in modifying mode)
    pub fn navigate_extreme_left(&mut self) {
        if !self.is_modifying {
            if self.inspect_mode {
                // Page left in inspect mode
                for _ in 0..10 {
                    self.inspect_scroll_up();
                }
            } else {
                // Go to leftmost column in table viewer
                self.scroll_x = 0;
                // Set selected column to first visible column
                if let Some(ref data) = self.table_data {
                    self.selected_column = data.columns.first().cloned();
                }
            }
        }
    }

    pub fn navigate_extreme_right(&mut self) {
        if !self.is_modifying {
            if self.inspect_mode {
                // Page right in inspect mode
                let (max_rows, visible_rows) = (20, 10); // Estimates - UI will provide better values
                for _ in 0..10 {
                    self.inspect_scroll_down(max_rows, visible_rows);
                }
            } else {
                // Go to rightmost column in table viewer
                self.selected_column = self.get_last_visible_column();
                // Estimate visible columns and scroll to show rightmost
                let total_cols = self.get_column_names().len();
                let estimated_visible_cols = 5;
                if total_cols > estimated_visible_cols {
                    self.scroll_x = total_cols - estimated_visible_cols;
                }
            }
        }
    }

    pub fn navigate_extreme_up(&mut self) {
        if !self.is_modifying {
            if self.inspect_mode {
                // Go to top in inspect mode
                if matches!(self.inspect_active_section, InspectSection::Schema) {
                    self.inspect_selected_row = 0;
                    self.inspect_schema_scroll_y = 0;
                } else {
                    self.inspect_stats_scroll_y = 0;
                }
            } else {
                // Go to first row in table viewer
                self.selected_row = 0;
                self.scroll_y = 0;
            }
        }
    }

    pub fn navigate_extreme_down(&mut self) {
        if !self.is_modifying {
            if self.inspect_mode {
                // Go to bottom in inspect mode
                if matches!(self.inspect_active_section, InspectSection::Schema) {
                    if let Some(ref data) = self.table_data {
                        let total_cols = data.columns.len();
                        if total_cols > 0 {
                            self.inspect_selected_row = total_cols - 1;
                            // Scroll to show bottom
                            let estimated_visible_rows = 10;
                            if total_cols > estimated_visible_rows {
                                self.inspect_schema_scroll_y = total_cols - estimated_visible_rows;
                            }
                        }
                    }
                } else {
                    // Go to bottom of statistics - estimate scroll
                    let estimated_max_rows = 20;
                    let estimated_visible_rows = 10;
                    if estimated_max_rows > estimated_visible_rows {
                        self.inspect_stats_scroll_y = estimated_max_rows - estimated_visible_rows;
                    }
                }
            } else if let Some(ref data) = self.table_data {
                // Go to last row in table viewer
                let total_rows = data.rows.len();
                if total_rows > 0 {
                    self.selected_row = total_rows - 1;
                    // Scroll to show bottom
                    let estimated_visible_rows = 10;
                    if total_rows > estimated_visible_rows {
                        self.scroll_y = total_rows - estimated_visible_rows;
                    }
                }
            }
        }
    }

    // Column name/index helper methods
    pub fn get_column_names(&self) -> Vec<String> {
        // Always return the virtual column order (which respects reordering)
        // This ensures all column operations work with the current display order
        self.get_virtual_column_order()
    }
    
    pub fn get_original_column_names(&self) -> Vec<String> {
        // Return the original database column order for SQL generation
        self.original_column_names.clone()
    }

    pub fn get_column_index_by_name(&self, column_name: &str) -> Option<usize> {
        self.get_column_names().iter().position(|name| name == column_name)
    }

    pub fn get_column_name_by_index(&self, index: usize) -> Option<String> {
        self.get_column_names().get(index).cloned()
    }

    pub fn get_selected_column_index(&self) -> Option<usize> {
        self.selected_column.as_ref()
            .and_then(|name| self.get_column_index_by_name(name))
    }

    pub fn get_next_visible_column(&self, current_column: &str) -> Option<String> {
        let column_names = self.get_column_names();
        if let Some(current_idx) = column_names.iter().position(|name| name == current_column) {
            for idx in (current_idx + 1)..column_names.len() {
                if let Some(name) = column_names.get(idx) {
                    if !self.is_column_hidden_by_name(name) {
                        return Some(name.clone());
                    }
                }
            }
        }
        None
    }

    pub fn get_prev_visible_column(&self, current_column: &str) -> Option<String> {
        let column_names = self.get_column_names();
        if let Some(current_idx) = column_names.iter().position(|name| name == current_column) {
            for idx in (0..current_idx).rev() {
                if let Some(name) = column_names.get(idx) {
                    if !self.is_column_hidden_by_name(name) {
                        return Some(name.clone());
                    }
                }
            }
        }
        None
    }

    pub fn get_first_visible_column(&self) -> Option<String> {
        let column_names = self.get_column_names();
        column_names.into_iter().find(|name| !self.is_column_hidden_by_name(name))
    }

    pub fn get_last_visible_column(&self) -> Option<String> {
        let column_names = self.get_column_names();
        for name in column_names.iter().rev() {
            if !self.is_column_hidden_by_name(name) {
                return Some(name.clone());
            }
        }
        None
    }

    // Column hiding methods
    pub fn toggle_column_visibility(&mut self) {
        if let Some(table_name) = self.selected_table.clone() {
            let column_name = if self.inspect_mode {
                // In inspect mode, get column name by row index
                self.get_column_name_by_index(self.inspect_selected_row)
            } else {
                // In table viewer, use selected column
                self.selected_column.clone()
            };
            
            if let Some(col_name) = column_name {
                let was_hidden = self.is_column_hidden_by_name(&col_name);
                
                // Get or create the hidden columns set for this table
                let hidden_set = self.hidden_columns.entry(table_name.clone()).or_default();
                
                if was_hidden {
                    // Column is hidden, show it
                    hidden_set.remove(&col_name);
                } else {
                    // Column is visible, hide it
                    hidden_set.insert(col_name.clone());
                }
                
                // Clean up empty sets
                if hidden_set.is_empty() {
                    self.hidden_columns.remove(&table_name);
                }
                
                // Auto-select next visible column when hiding current selection
                if !was_hidden && Some(&col_name) == self.selected_column.as_ref() {
                    if let Some(next_col) = self.get_next_visible_column(&col_name) {
                        self.selected_column = Some(next_col);
                    } else if let Some(prev_col) = self.get_prev_visible_column(&col_name) {
                        self.selected_column = Some(prev_col);
                    } else {
                        self.selected_column = None;
                    }
                }
            }
        }
    }

    pub fn is_column_hidden_by_name(&self, column_name: &str) -> bool {
        if let Some(table_name) = &self.selected_table {
            self.hidden_columns
                .get(table_name)
                .is_some_and(|hidden_set| hidden_set.contains(column_name))
        } else {
            false
        }
    }

    pub fn is_column_hidden(&self, column_index: usize) -> bool {
        if let Some(column_name) = self.get_column_name_by_index(column_index) {
            self.is_column_hidden_by_name(&column_name)
        } else {
            false
        }
    }

    pub fn get_visible_column_names(&self) -> Vec<String> {
        let column_names = self.get_column_names();
        column_names.into_iter()
            .filter(|name| !self.is_column_hidden_by_name(name))
            .collect()
    }

    pub fn get_visible_columns(&self) -> Vec<usize> {
        // This method returns the indices of visible columns in their virtual order
        let virtual_order = self.get_virtual_column_order();
        let original_names = self.get_original_column_names();
        
        virtual_order.iter()
            .enumerate()
            .filter(|(_, name)| !self.is_column_hidden_by_name(name))
            .filter_map(|(virtual_idx, name)| {
                // Find the original index of this column name
                original_names.iter().position(|orig_name| orig_name == name)
                    .map(|_| virtual_idx)
            })
            .collect()
    }

    pub fn get_hidden_columns(&self) -> std::collections::HashSet<String> {
        if let Some(table_name) = &self.selected_table {
            self.hidden_columns
                .get(table_name)
                .cloned()
                .unwrap_or_default()
        } else {
            std::collections::HashSet::new()
        }
    }

    pub fn clear_hidden_columns(&mut self) {
        if let Some(table_name) = &self.selected_table {
            self.hidden_columns.remove(table_name);
        }
    }
}