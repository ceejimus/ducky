use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph, Wrap, Table, Row, Cell, TableState},
    Frame,
};

use crate::actions::ActionLogger;
use crate::app::state::{ApplicationState, NavigationPanel, AppState, TableCreationStep};
use crate::db::query::SortDirection;
use crate::db::DatabaseManager;
use crate::workflows::DatabaseWorkflows;
use crate::state::{StateManager, StateTransition, StateKey};

mod file_browser;
use file_browser::{render_file_browser_popup, FileBrowser, detect_file_type, FileType};

pub struct App {
    state: ApplicationState,
    database_manager: DatabaseManager,
    file_browser: Option<FileBrowser>,
    show_file_browser: bool,
    action_logger: ActionLogger,
    // State pattern manager
    state_manager: StateManager,
}

impl App {
    pub fn new() -> Self {
        Self::new_with_database(None)
    }

    pub fn new_with_database(database_path: Option<std::path::PathBuf>) -> Self {
        let mut database_manager = DatabaseManager::new();

        // Initialize ActionLogger
        let mut action_logger = ActionLogger::new().unwrap_or_else(|e| {
            panic!("Warning: Failed to initialize action logger: {e}");
            // Create a dummy logger that doesn't write to file
            // ActionLogger::new().unwrap()
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

        let mut app = Self {
            state: ApplicationState::new(),
            database_manager,
            file_browser: None,
            show_file_browser: false,
            action_logger,
            // Initialize state pattern manager
            state_manager: StateManager::new(),
        };
        
        // Initialize state pattern components
        app.init_state_pattern();
        
        // If we loaded a database via CLI, update the application state to reflect the selection
        if database_path.is_some() {
            if let Some(current_db) = app.database_manager.get_current_database() {
                app.state.select_database(current_db.to_string());
            }
        }
        
        app
    }

    fn init_state_pattern(&mut self) {
        // Get current database for initialization
        let current_db = self.database_manager.get_current_database().map(|s| s.to_string());
        
        // Initialize the context with current database
        self.state_manager.get_context_mut().current_database = current_db;
        
        // Initialize state manager
        self.state_manager.initialize(&mut self.database_manager);
    }

    fn sync_from_state_manager(&mut self) {
        // Sync notifications from state manager to main app state
        let new_notifications = self.state_manager.take_notifications();
        for notification in new_notifications {
            self.state.add_notification(notification);
        }
        
        // Sync active panel from state pattern to legacy UI
        let state_manager_active_state = self.state_manager.get_active_state().clone();
        if let Some(expected_panel) = self.state_key_to_navigation_panel(state_manager_active_state) {
            if self.state.active_panel != expected_panel {
                log::debug!("UI: syncing active panel from {:?} to {:?}", self.state.active_panel, expected_panel);
                self.state.active_panel = expected_panel;
            }
        }
        
        // Sync database selection changes
        let context = self.state_manager.get_context();
        if let Some(current_db) = &context.current_database {
            if self.state.selected_database.as_ref() != Some(current_db) {
                self.state.select_database(current_db.clone());
            }
        }
        
        // Sync table selection changes
        if let Some(current_table) = &context.selected_table {
            if self.state.selected_table.as_ref() != Some(current_table) {
                self.state.selected_table = Some(current_table.clone());
            }
        }
        
        // Sync table data changes
        if let Some(table_data) = &context.table_data {
            self.state.table_data = Some(table_data.clone());
            
            // Initialize column order when table data is synced (matching legacy behavior)
            self.state.initialize_column_order(table_data.columns.clone());
            
            // Initialize selected column if not set (needed for navigation and highlighting)
            if self.state.selected_column.is_none() && !table_data.columns.is_empty() {
                self.state.selected_column = Some(table_data.columns[0].clone());
            }
        }
    }

    fn sync_to_state_manager(&mut self) {
        // Sync flash state from ApplicationState to AppContext
        let context = self.state_manager.get_context_mut();
        context.panel_flash_timer = self.state.panel_flash_timer;
        context.flash_duration_ms = self.state.flash_duration_ms;
        
        // Map current NavigationPanel to StateKey and ensure StateManager is in correct state
        if let Some(state_key) = self.navigation_panel_to_state_key(self.state.active_panel.clone()) {
            self.state_manager.enter_state(state_key, &mut self.database_manager);
        }
    }
    
    fn navigation_panel_to_state_key(&self, panel: NavigationPanel) -> Option<StateKey> {
        match panel {
            NavigationPanel::DatabaseList => Some(StateKey::DatabaseSelect),
            NavigationPanel::TableList => Some(StateKey::TableSelect),
            _ => None, // Other panels not implemented in state pattern yet
        }
    }
    
    fn state_key_to_navigation_panel(&self, state_key: StateKey) -> Option<NavigationPanel> {
        match state_key {
            StateKey::DatabaseSelect => Some(NavigationPanel::DatabaseList),
            StateKey::TableSelect => Some(NavigationPanel::TableList),
            _ => None, // Other states not implemented in legacy UI
        }
    }


    pub fn handle_key(&mut self, key: KeyEvent) {
        // Handle file browser first if it's open
        if self.show_file_browser {
            match key.code {
                KeyCode::Esc => {
                    self.show_file_browser = false;
                    self.file_browser = None;
                }
                _ => {
                    if let Some(ref mut browser) = self.file_browser {
                        if let Ok(Some(selected_path)) = browser.handle_key(key) {
                            // Detect file type and route appropriately
                            if let Some(file_type) = detect_file_type(&selected_path) {
                                match file_type {
                                    FileType::Database => {
                                        // Handle database file connection
                                        let mut workflows = DatabaseWorkflows::new(
                                            &mut self.database_manager,
                                            &mut self.action_logger,
                                            &mut self.state,
                                        );
                                        let _ = workflows.select_file(selected_path);
                                        self.state_manager.sync_all_states(&self.database_manager);
                                    }
                                    FileType::DataFile(_) => {
                                        // Handle data file import
                                        if self.state.is_creating_table && self.state.table_creation_step == TableCreationStep::SelectingFile {
                                            let table_name = self.state.new_table_name.clone();
                                            self.state.set_importing_data();
                                            
                                            let mut workflows = DatabaseWorkflows::new(
                                                &mut self.database_manager,
                                                &mut self.action_logger,
                                                &mut self.state,
                                            );
                                            
                                            match workflows.import_file_to_table(selected_path, table_name) {
                                                Ok(_) => {
                                                    self.state.complete_table_creation(true);
                                                    // Force refresh to ensure UI has latest table data
                                                    self.refresh_current_database();
                                                    self.state_manager.sync_all_states(&self.database_manager);
                                                    // Fetch data for the newly created table
                                                    self.fetch_table_data();
                                                }
                                                Err(e) => {
                                                    self.state.show_error(format!("Import failed: {e}"));
                                                    self.state.complete_table_creation(false);
                                                }
                                            }
                                        } else {
                                            self.state.show_error("Please start table creation first (press 'i')".to_string());
                                        }
                                    }
                                }
                            } else {
                                self.state.show_error("Unsupported file type".to_string());
                            }

                            self.show_file_browser = false;
                            self.file_browser = None;
                        }
                    }
                }
            }
            return;
        }


        // Handle view name input
        if self.state.is_entering_view_name {
            match key.code {
                KeyCode::Esc => {
                    self.state.cancel_view_name_input();
                }
                KeyCode::Enter => {
                    if !self.state.new_view_name.trim().is_empty() {
                        self.create_view_from_current_state();
                        self.state.cancel_view_name_input();
                    }
                }
                KeyCode::Backspace => {
                    self.state.remove_char_from_view_name();
                }
                KeyCode::Char(c) => {
                    self.state.add_char_to_view_name(c);
                }
                _ => {}
            }
            return;
        }

        // Handle search mode input
        if self.state.is_searching {
            match key.code {
                KeyCode::Esc => {
                    self.state.cancel_search();
                }
                KeyCode::Enter => {
                    if self.state.finalize_search() {
                        // Filter was applied, refresh data without limit to show all results
                        self.fetch_table_data_preserve_column_with_limit(false);
                    }
                }
                KeyCode::Backspace => {
                    self.state.remove_char_from_search();
                    self.validate_search_syntax();
                }
                KeyCode::Char(c) => {
                    self.state.add_char_to_search(c);
                    self.validate_search_syntax();
                }
                _ => {}
            }
            return;
        }


        // Handle table creation input
        if self.state.is_creating_table && self.state.table_creation_step == TableCreationStep::EnteringTableName {
            match key.code {
                KeyCode::Esc => {
                    self.state.cancel_table_creation();
                }
                KeyCode::Enter => {
                    if !self.state.new_table_name.trim().is_empty() {
                        self.state.confirm_table_name();
                        self.open_file_browser();
                    }
                }
                KeyCode::Backspace => {
                    self.state.remove_char_from_table_name();
                }
                KeyCode::Char(c) => {
                    self.state.add_char_to_table_name(c);
                }
                _ => {}
            }
            return;
        }


        // Try state pattern key handling first
        log::debug!("UI: trying state pattern for key: {:?}, active_panel: {:?}", key.code, self.state.active_panel);
        if let Some(transition) = self.state_manager.handle_key_event(key, &mut self.database_manager) {
            log::debug!("UI: state pattern returned transition: {:?}", transition);
            // Event was handled by state pattern - always sync state
            self.sync_from_state_manager();
            
            // Handle specific transition types
            match transition {
                StateTransition::Exit => {
                    // Handle exit request
                    return; // Exit the application - this will be handled by the main loop
                }
                StateTransition::Stay | StateTransition::Push(_) | StateTransition::Pop => {
                    // These are handled internally by state manager
                    return;
                }
                StateTransition::To(state_key) => {
                    // Legacy panel navigation for unimplemented state pattern panels
                    log::debug!("UI: handling To({:?}) - navigating to legacy panel", state_key);
                    match state_key {
                        StateKey::TableDataViewer => {
                            self.state.set_active_panel(NavigationPanel::MainContent);
                            self.sync_to_state_manager();
                        }
                        StateKey::TableInspector => {
                            self.state.set_active_panel(NavigationPanel::MainContent);
                            self.sync_to_state_manager();
                        }
                        _ => {
                            // Should not happen - state manager should handle implemented states internally
                            panic!("UI: unexpected To({:?}) transition for implemented state", state_key);
                        }
                    }
                    return;
                }
            }
        } else {
            log::debug!("UI: state pattern returned None, falling back to legacy handling for key: {:?}", key.code);
        }

        // Normal key handling (legacy fallback)
        match key.code {
            KeyCode::Tab => {
                if self.state.inspect_mode {
                    // In inspect mode: cycle between schema and statistics sections
                    self.state.inspect_cycle_section();
                } else {
                    self.state.next_panel();
                    self.sync_to_state_manager();
                }
            }
            KeyCode::BackTab => {
                self.state.prev_panel();
                self.sync_to_state_manager();
            }
            KeyCode::Esc => {
                if self.state.is_modifying {
                    // Cancel modifying mode
                    self.state.cancel_modifying();
                    self.fetch_table_data_preserve_column();
                    // No popup - will show in status bar instead
                } else if self.state.inspect_mode {
                    // Exit inspect mode
                    self.state.exit_inspect_mode();
                }
                // Note: Could add other escape behaviors here in the future
            }
            KeyCode::Up => self.handle_vim_up(),
            KeyCode::Down => self.handle_vim_down(),
            KeyCode::Left => self.handle_vim_left(),
            KeyCode::Right => self.handle_vim_right(),
            KeyCode::Enter => self.handle_enter(),
            KeyCode::Char('?') => self.show_help(),
            KeyCode::Char('k') => self.handle_vim_up(),
            KeyCode::Char('j') => self.handle_vim_down(),
            KeyCode::Char('h') => self.handle_vim_left(),
            KeyCode::Char('l') => self.handle_vim_right(),
            KeyCode::Char('K') => self.handle_extreme_up(),
            KeyCode::Char('J') => self.handle_extreme_down(),
            KeyCode::Char('H') => self.handle_extreme_left(),
            KeyCode::Char('L') => self.handle_extreme_right(),
            KeyCode::Char('m') => self.handle_modify_mode(),
            KeyCode::Char('i') => {
                match self.state.active_panel {
                    NavigationPanel::TableList => {
                        // In table list: start import/table creation
                        self.start_table_creation();
                    }
                    NavigationPanel::MainContent => {
                        // In table viewer: enter inspect mode
                        if self.state.table_data.is_some() {
                            self.state.enter_inspect_mode();
                        }
                    }
                    _ => {}
                }
            }
            KeyCode::Char('o') => {
                if self.state.is_modifying {
                    // In modifying mode: toggle column visibility
                    self.state.toggle_column_visibility();
                    self.fetch_table_data_preserve_column();
                } else {
                    // Normal mode: open file browser
                    self.open_file_browser();
                }
            }
            KeyCode::Char('v') => {
                // Start save view (only in table viewer or inspector mode)
                if (self.state.active_panel == NavigationPanel::MainContent || self.state.inspect_mode) && 
                   self.state.table_data.is_some() && self.state.selected_table.is_some() {
                    self.state.start_view_name_input();
                } else {
                    self.state.show_error("No table selected to create view from".to_string());
                }
            }
            KeyCode::Char('q') => {
                // Quit handled by main loop
            }
            KeyCode::Char('a') => {
                // Toggle column in sort chain as ascending (only in table viewer)
                if self.state.active_panel == NavigationPanel::MainContent && self.state.table_data.is_some() {
                    self.state.toggle_in_sort_chain(true);
                    self.fetch_table_data_preserve_column();
                }
            }
            KeyCode::Char('A') => {
                // Toggle column in sort chain as descending (only in table viewer)
                if self.state.active_panel == NavigationPanel::MainContent && self.state.table_data.is_some() {
                    self.state.toggle_in_sort_chain(false);
                    self.fetch_table_data_preserve_column();
                }
            }
            KeyCode::Char('c') => {
                // Clear all sorting (only in table viewer)
                if self.state.active_panel == NavigationPanel::MainContent && self.state.table_data.is_some() {
                    self.state.clear_sort();
                    self.fetch_table_data_preserve_column();
                }
            }
            KeyCode::Char('f') => {
                // Start column filter mode (only in table viewer)
                if self.state.active_panel == NavigationPanel::MainContent && self.state.table_data.is_some() {
                    if let Some(selected_idx) = self.state.get_selected_column_index() {
                        self.state.start_column_search(selected_idx);
                    }
                }
            }
            KeyCode::Char('F') => {
                // Clear filter on selected column (only in table viewer)
                if self.state.active_panel == NavigationPanel::MainContent && self.state.table_data.is_some() {
                    if let Some(selected_col) = self.state.selected_column.clone() {
                        self.state.clear_column_filter(&selected_col);
                        self.fetch_table_data_preserve_column_with_limit(false);
                    }
                }
            }
            KeyCode::Char('1') => {
                self.state.set_left_panel(NavigationPanel::DatabaseList);
                self.sync_to_state_manager();
            }
            KeyCode::Char('2') => {
                self.state.set_left_panel(NavigationPanel::TableList);
                self.sync_to_state_manager();
            }
            KeyCode::Char('3') => {
                self.state.set_active_panel(NavigationPanel::MainContent);
                self.sync_to_state_manager();
            }
            _ => {}
        }
    }


    // Vim navigation keys for UI navigation and modal modification
    fn handle_vim_up(&mut self) {
        // Handle modifying mode for inspect view
        if self.state.is_modifying && self.state.inspect_mode {
            if matches!(self.state.inspect_active_section, crate::app::state::InspectSection::Schema) {
                let selected_row = self.state.inspect_selected_row;
                if selected_row > 0 {
                    let target_row = selected_row - 1;
                    if self.state.reorder_column(selected_row, target_row) {
                        self.fetch_table_data_preserve_column();
                        self.state.inspect_selected_row = target_row;
                    }
                }
            }
            return;
        }
        
        // Handle inspect mode navigation
        if self.state.inspect_mode {
            // Normal navigation in inspect mode
            if matches!(self.state.inspect_active_section, crate::app::state::InspectSection::Schema) {
                // Move selection up in columns view
                self.state.inspect_move_selection_up();
            } else {
                // Scroll up in statistics view
                self.state.inspect_scroll_up();
            }
            return;
        }

        match self.state.active_panel {
            NavigationPanel::MainContent => {
                // Move selected row up
                if let Some(ref _data) = self.state.table_data {
                    let _visible_rows = 10; // Approximate - could be calculated from area
                    self.state.move_selected_up();
                }
            }
            _ => {}
        }
    }

    fn handle_vim_down(&mut self) {
        // Handle modifying mode for inspect view
        if self.state.is_modifying && self.state.inspect_mode {
            if matches!(self.state.inspect_active_section, crate::app::state::InspectSection::Schema) {
                let total_cols = self.state.get_virtual_column_order().len();
                let selected_row = self.state.inspect_selected_row;
                
                if selected_row + 1 < total_cols {
                    let target_row = selected_row + 1;
                    if self.state.reorder_column(selected_row, target_row) {
                        self.fetch_table_data_preserve_column();
                        self.state.inspect_selected_row = target_row;
                    }
                }
            }
            return;
        }
        
        // Handle inspect mode navigation
        if self.state.inspect_mode {
            // Normal navigation in inspect mode
            if matches!(self.state.inspect_active_section, crate::app::state::InspectSection::Schema) {
                // Move selection down in columns view (use total columns, not just visible)
                let total_columns = self.state.get_virtual_column_order().len();
                let (_max_rows, visible_rows) = self.calculate_inspect_scroll_bounds();
                self.state.inspect_move_selection_down(total_columns, visible_rows);
            } else {
                // Scroll down in statistics view
                let (max_rows, visible_rows) = self.calculate_inspect_scroll_bounds();
                self.state.inspect_scroll_down(max_rows, visible_rows);
            }
            return;
        }

        match self.state.active_panel {
            NavigationPanel::MainContent => {
                // Move selected row down
                if let Some(ref data) = self.state.table_data {
                    // Calculate visible rows accounting for word wrapping using cached area height
                    let area_height = self.state.last_table_area_height;
                    let visible_rows = self.calculate_visible_data_rows_from_height(area_height, data);
                    self.state.move_selected_down(data.rows.len(), visible_rows);
                }
            }
            _ => {}
        }
    }

    fn handle_vim_left(&mut self) {
        // Handle modifying mode for table viewer - move selected column left
        if self.state.is_modifying && matches!(self.state.active_panel, NavigationPanel::MainContent) && !self.state.inspect_mode {
            if let Some(selected_column) = &self.state.selected_column {
                if let Some(current_index) = self.state.get_column_index_by_name(selected_column) {
                    if current_index > 0 {
                        let target_index = current_index - 1;
                        if self.state.reorder_column(current_index, target_index) {
                            self.fetch_table_data_preserve_column();
                        }
                    }
                }
            }
            return;
        }
        
        // Handle inspect mode navigation - treat left as page up
        if self.state.inspect_mode {
            // Page up: scroll up by multiple rows
            for _ in 0..5 {
                self.state.inspect_scroll_up();
            }
            return;
        }

        match self.state.active_panel {
            NavigationPanel::MainContent => {
                // Move selected column left
                if self.state.table_data.is_some() {
                    self.state.move_selected_left();
                }
            }
            NavigationPanel::TableList => {
                // From table list, go to database widget  
                self.state.set_left_panel(NavigationPanel::DatabaseList);
                self.sync_to_state_manager();
            }
            NavigationPanel::DatabaseList => {
                // From database list, underflow to table list
                self.state.set_left_panel(NavigationPanel::TableList);
                self.sync_to_state_manager();
            }
            _ => {}
        }
    }

    fn handle_vim_right(&mut self) {
        // Handle modifying mode for table viewer - move selected column right
        if self.state.is_modifying && matches!(self.state.active_panel, NavigationPanel::MainContent) && !self.state.inspect_mode {
            if let Some(selected_column) = &self.state.selected_column {
                if let Some(current_index) = self.state.get_column_index_by_name(selected_column) {
                    let virtual_order = self.state.get_virtual_column_order();
                    if current_index < virtual_order.len() - 1 {
                        let target_index = current_index + 1;
                        if self.state.reorder_column(current_index, target_index) {
                            self.fetch_table_data_preserve_column();
                        }
                    }
                }
            }
            return;
        }
        
        // Handle inspect mode navigation - treat right as page down
        if self.state.inspect_mode {
            // Page down: scroll down by multiple rows
            let (max_rows, visible_rows) = self.calculate_inspect_scroll_bounds();
            for _ in 0..5 {
                self.state.inspect_scroll_down(max_rows, visible_rows);
            }
            return;
        }

        match self.state.active_panel {
            NavigationPanel::MainContent => {
                // Move selected column right
                if let Some(ref data) = self.state.table_data {
                    // Calculate visible columns to pass to method
                    let visible_cols = 5; // Approximate - could be calculated from area
                    self.state.move_selected_right(data.columns.len(), visible_cols);
                }
            }
            _ => {}
        }
    }

    // Extreme movement handlers (capital letters)
    fn handle_extreme_up(&mut self) {
        if self.state.is_modifying {
            if self.state.move_column_extreme_up() {
                self.fetch_table_data_preserve_column();
            }
        } else {
            self.state.navigate_extreme_up();
        }
    }

    fn handle_extreme_down(&mut self) {
        if self.state.is_modifying {
            if self.state.move_column_extreme_down() {
                self.fetch_table_data_preserve_column();
            }
        } else {
            self.state.navigate_extreme_down();
        }
    }

    fn handle_extreme_left(&mut self) {
        if self.state.is_modifying {
            if self.state.move_column_extreme_left() {
                self.fetch_table_data_preserve_column();
            }
        } else {
            self.state.navigate_extreme_left();
        }
    }

    fn handle_extreme_right(&mut self) {
        if self.state.is_modifying {
            if self.state.move_column_extreme_right() {
                self.fetch_table_data_preserve_column();
            }
        } else {
            self.state.navigate_extreme_right();
        }
    }

    fn handle_modify_mode(&mut self) {
        if ((self.state.active_panel == NavigationPanel::MainContent && self.state.table_data.is_some()) ||
           (self.state.inspect_mode && matches!(self.state.inspect_active_section, crate::app::state::InspectSection::Schema)))
           && !self.state.is_modifying {
            self.state.start_modifying();
            // No popup - will show in status bar instead
        }
    }



    fn refresh_current_database(&mut self) {
        if let Some(current_db) = self.database_manager.get_current_database() {
            let current_db = current_db.to_string();
            if let Err(e) = self.database_manager.refresh_database(&current_db) {
                self.state.show_error(format!("Failed to refresh database: {e}"));
            }
        }
    }

    fn render_table_widget(&self, f: &mut Frame, area: Rect, data: &crate::db::query::QueryResult, title: &str) {
        let border_style = self.get_panel_border_style(NavigationPanel::MainContent);

        // Create mapping from data column indices to original virtual column indices
        // Since hidden columns are filtered out in SQL, data.columns only contains visible columns
        let visible_columns = self.state.get_visible_columns();
        
        // Calculate visible rows and columns
        let available_height = area.height.saturating_sub(3) as usize; // Subtract borders and header
        let start_row = self.state.scroll_y;
        let end_row = (start_row + available_height).min(data.rows.len());
        
        // Calculate column constraints and visible columns
        let mut constraints = Vec::new();
        let mut visible_cols = Vec::new();
        let available_width = area.width.saturating_sub(2) as usize; // Subtract borders
        let start_col = self.state.scroll_x;
        let mut used_width = 0;
        
        // Calculate individual column widths based on content
        let min_col_width = 8;
        let mut column_widths = Vec::new();
        
        for (data_col_idx, col_name) in data.columns.iter().enumerate() {
            // Map data column index to original virtual column index
            let virtual_col_idx = visible_columns.get(data_col_idx).copied().unwrap_or(data_col_idx);
            
            // Calculate header width using the final header text (including sort indicators)
            let final_header_text = self.get_final_header_text(virtual_col_idx, col_name);
            let header_width = final_header_text.chars().count();
            let mut max_data_width = 0;
            
            // Calculate max data width for visible rows
            if self.state.is_column_expanded(virtual_col_idx) {
                // For expanded columns, check all visible rows for more accurate width
                for row in data.rows[start_row..end_row].iter() {
                    if let Some(cell) = row.get(data_col_idx) {
                        max_data_width = max_data_width.max(cell.chars().count());
                    }
                }
            } else {
                // For normal columns, sample first 10 rows for performance
                for row in data.rows.iter().take(10) {
                    if let Some(cell) = row.get(data_col_idx) {
                        max_data_width = max_data_width.max(cell.chars().count());
                    }
                }
            }
            
            let col_width = if self.state.is_column_expanded(virtual_col_idx) {
                // Expanded column: fit content up to max of 50 characters
                header_width.max(max_data_width).max(min_col_width).min(50)
            } else {
                // Normal column: limit to 25 characters
                header_width.max(max_data_width).max(min_col_width).min(25)
            };
            
            column_widths.push(col_width);
        }

        // Determine visible columns
        for i in start_col..data.columns.len() {
            let col_width = column_widths.get(i).unwrap_or(&min_col_width);
            if used_width + col_width <= available_width {
                constraints.push(Constraint::Length(*col_width as u16));
                visible_cols.push(i);
                used_width += col_width;
            } else {
                break;
            }
        }

        // Ensure we show at least one column
        if visible_cols.is_empty() && start_col < data.columns.len() {
            visible_cols.push(start_col);
            constraints.push(Constraint::Length(min_col_width as u16));
        }

        // Create header with styling for selected column and sort indicators
        let header_cells: Vec<Cell> = visible_cols.iter().map(|&data_col_idx| {
            let cell_content = &data.columns[data_col_idx];
            let col_width = column_widths.get(data_col_idx).unwrap_or(&min_col_width);
            
            // Map data column index to virtual column index
            let virtual_col_idx = visible_columns.get(data_col_idx).copied().unwrap_or(data_col_idx);
            
            // Get the complete header text with all indicators
            let header_with_sort = self.get_final_header_text(virtual_col_idx, cell_content);
            
            let text = if self.state.is_column_expanded(virtual_col_idx) {
                // For expanded columns, show full header without truncation
                header_with_sort
            } else {
                truncate_text(&header_with_sort, *col_width)
            };
            
            if Some(virtual_col_idx) == self.state.get_selected_column_index() {
                Cell::from(text).style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            } else {
                Cell::from(text)
            }
        }).collect();

        // Create data rows with text wrapping support for expanded columns
        let mut rows: Vec<Row> = Vec::new();
        
        for (display_idx, row) in data.rows[start_row..end_row].iter().enumerate() {
            let actual_row_idx = start_row + display_idx;
            let is_selected_row = actual_row_idx == self.state.selected_row;
            
            // First, prepare wrapped content for all cells in this row
            let mut cell_lines: Vec<Vec<String>> = Vec::new();
            let mut max_lines = 1;
            
            for &data_col_idx in &visible_cols {
                let empty_string = String::new();
                let cell_content = row.get(data_col_idx).unwrap_or(&empty_string);
                let col_width = column_widths.get(data_col_idx).unwrap_or(&min_col_width);
                
                // Map data column index to virtual column index
                let virtual_col_idx = visible_columns.get(data_col_idx).copied().unwrap_or(data_col_idx);
                
                let lines = if self.state.is_column_expanded(virtual_col_idx) {
                    wrap_text(cell_content, *col_width)
                } else {
                    vec![truncate_text(cell_content, *col_width)]
                };
                
                max_lines = max_lines.max(lines.len());
                cell_lines.push(lines);
            }
            
            // Create multiple rows if any cell has wrapped content
            for line_idx in 0..max_lines {
                let cells: Vec<Cell> = visible_cols.iter().enumerate().map(|(visible_idx, &data_col_idx)| {
                    let line_text = cell_lines.get(visible_idx)
                        .and_then(|lines| lines.get(line_idx))
                        .unwrap_or(&String::new())
                        .clone();
                    
                    // Map data column index to virtual column index
                    let virtual_col_idx = visible_columns.get(data_col_idx).copied().unwrap_or(data_col_idx);
                    
                    // Check if this is the current cell (intersection of selected row and column)
                    let is_current_cell = is_selected_row && Some(virtual_col_idx) == self.state.get_selected_column_index();
                    
                    if is_current_cell {
                        // Highlight current cell with light gray background and inverted text for readability
                        Cell::from(line_text).style(Style::default().bg(Color::Gray).fg(Color::Black).add_modifier(Modifier::BOLD))
                    } else if is_selected_row {
                        // Bold selected row
                        Cell::from(line_text).style(Style::default().add_modifier(Modifier::BOLD))
                    } else if Some(virtual_col_idx) == self.state.get_selected_column_index() {
                        // Subtle highlight for selected column
                        Cell::from(line_text).style(Style::default().fg(Color::Gray))
                    } else {
                        Cell::from(line_text)
                    }
                }).collect();
                
                rows.push(Row::new(cells));
            }
        }

        let table = Table::new(rows, constraints)
            .header(Row::new(header_cells).height(1))
            .block(Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(border_style))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol(">>")
            .column_spacing(1);

        f.render_widget(table, area);
    }

    // Calculate how many complete data rows actually fit in the available height
    // accounting for word wrapping in expanded columns
    fn calculate_visible_data_rows(&self, area: Rect, data: &crate::db::query::QueryResult) -> usize {
        let available_height = area.height.saturating_sub(3) as usize; // Subtract borders and header
        let start_row = self.state.scroll_y;
        
        if data.rows.is_empty() {
            return 0;
        }

        // Calculate column widths (simplified version of what's in render_table_widget)
        let min_col_width = 8;
        let mut column_widths = Vec::new();
        for (i, col_name) in data.columns.iter().enumerate() {
            let final_header_text = self.get_final_header_text(i, col_name);
            let header_width = final_header_text.chars().count();
            
            let col_width = if self.state.is_column_expanded(i) {
                header_width.max(min_col_width).min(50)
            } else {
                header_width.max(min_col_width).min(25)
            };
            column_widths.push(col_width);
        }

        // Calculate visible columns (simplified)
        let available_width = area.width.saturating_sub(2) as usize;
        let start_col = self.state.scroll_x;
        let mut visible_cols = Vec::new();
        let mut used_width = 0;
        
        for i in start_col..data.columns.len() {
            let col_width = column_widths.get(i).unwrap_or(&min_col_width);
            if used_width + col_width <= available_width {
                visible_cols.push(i);
                used_width += col_width;
            } else {
                break;
            }
        }

        // Ensure we show at least one column
        if visible_cols.is_empty() && start_col < data.columns.len() {
            visible_cols.push(start_col);
        }

        // Count how many complete data rows fit
        let mut used_height = 0;
        let mut visible_data_rows = 0;
        
        for row_idx in start_row..data.rows.len() {
            if row_idx >= data.rows.len() {
                break;
            }
            
            let row = &data.rows[row_idx];
            
            // Calculate max lines for this row (same logic as render_table_widget)
            let mut max_lines = 1;
            for &col_idx in &visible_cols {
                let empty_string = String::new();
                let cell_content = row.get(col_idx).unwrap_or(&empty_string);
                let col_width = column_widths.get(col_idx).unwrap_or(&min_col_width);
                
                let lines = if self.state.is_column_expanded(col_idx) {
                    wrap_text(cell_content, *col_width)
                } else {
                    vec![truncate_text(cell_content, *col_width)]
                };
                
                max_lines = max_lines.max(lines.len());
            }
            
            // Check if this row fits in remaining height
            if used_height + max_lines <= available_height {
                used_height += max_lines;
                visible_data_rows += 1;
            } else {
                break;
            }
        }
        
        visible_data_rows.max(1) // Always return at least 1
    }

    // Simplified version that only needs cached height (for navigation)
    fn calculate_visible_data_rows_from_height(&self, area_height: u16, data: &crate::db::query::QueryResult) -> usize {
        let available_height = area_height.saturating_sub(3) as usize; // Subtract borders and header
        
        if data.rows.is_empty() {
            return 0;
        }
        
        // If no expanded columns, each row is height 1
        if self.state.expanded_columns.is_empty() {
            return available_height.max(1);
        }
        
        // Calculate column widths similar to render_table_widget logic
        let min_col_width = 8;
        let mut column_widths = Vec::new();
        for (i, col_name) in data.columns.iter().enumerate() {
            let final_header_text = self.get_final_header_text(i, col_name);
            let header_width = final_header_text.chars().count();
            
            let col_width = if self.state.is_column_expanded(i) {
                header_width.max(min_col_width).min(50)
            } else {
                header_width.max(min_col_width).min(25)
            };
            column_widths.push(col_width);
        }

        // Calculate visible columns (simplified, assume all columns fit for navigation)
        let available_width = 100; // Reasonable assumption for navigation calculations
        let start_col = self.state.scroll_x;
        let mut visible_cols = Vec::new();
        let mut used_width = 0;
        
        for i in start_col..data.columns.len() {
            let col_width = column_widths.get(i).unwrap_or(&min_col_width);
            if used_width + col_width <= available_width {
                visible_cols.push(i);
                used_width += col_width;
            } else {
                break;
            }
        }
        
        // Ensure we show at least one column
        if visible_cols.is_empty() && start_col < data.columns.len() {
            visible_cols.push(start_col);
        }

        // Count how many complete data rows fit (same logic as calculate_visible_data_rows)
        let mut used_height = 0;
        let mut visible_data_rows = 0;
        let start_row = self.state.scroll_y;
        
        for row_idx in start_row..data.rows.len() {
            if row_idx >= data.rows.len() {
                break;
            }
            
            let row = &data.rows[row_idx];
            
            // Calculate max lines for this row
            let mut max_lines = 1;
            for &col_idx in &visible_cols {
                let empty_string = String::new();
                let cell_content = row.get(col_idx).unwrap_or(&empty_string);
                let col_width = column_widths.get(col_idx).unwrap_or(&min_col_width);
                
                let lines = if self.state.is_column_expanded(col_idx) {
                    wrap_text(cell_content, *col_width)
                } else {
                    vec![truncate_text(cell_content, *col_width)]
                };
                
                max_lines = max_lines.max(lines.len());
            }
            
            // Check if this row fits in remaining height
            if used_height + max_lines <= available_height {
                used_height += max_lines;
                visible_data_rows += 1;
            } else {
                break;
            }
        }
        
        visible_data_rows.max(1) // Always return at least 1
    }

    // Helper method to generate the complete header text for a column including all indicators
    // This ensures width calculations match the actual rendered text
    fn get_final_header_text(&self, _column_index: usize, column_name: &str) -> String {
        let mut header_text = column_name.to_string();
        
        // Add sort indicator if this column is in the sort chain
        if let Some((position, sort_spec)) = self.state.sort_columns
            .iter()
            .enumerate()
            .find(|(_, spec)| spec.column_name == column_name) {
            
            let sort_indicator = match sort_spec.direction {
                SortDirection::Ascending => "↑",
                SortDirection::Descending => "↓",
            };
            
            // Show order number for multi-column sorts (1-indexed for user readability)
            if self.state.sort_columns.len() > 1 {
                header_text = format!("{} {}^{}", header_text, sort_indicator, position + 1);
            } else {
                header_text = format!("{header_text} {sort_indicator}");
            }
        }
        
        // Add filter indicator if this column is filtered
        if self.state.is_column_filtered(column_name) {
            header_text = format!("{header_text} *");
        }
        
        header_text
    }

    fn fetch_table_data(&mut self) {
        if let (Some(_db), Some(table)) = (self.state.selected_database.clone(), self.state.selected_table.clone()) {
            if let Some(connection) = self.database_manager.get_current_connection() {
                // First get column names to enable sorting and virtual ordering
                let column_names = match self.get_table_column_names(connection, &table) {
                    Ok(names) => names,
                    Err(e) => {
                        self.state.show_error(format!("Failed to get column names: {e}"));
                        return;
                    }
                };

                // Initialize column order if needed
                self.state.initialize_column_order(column_names.clone());
                
                // Get visible columns in virtual order
                let visible_column_names = self.state.get_visible_column_names();
                
                // Use visible columns directly (they are already in virtual order)
                let ordered_columns = visible_column_names;

                // Build base SQL query with virtual column ordering
                let columns_sql = ordered_columns.join(", ");
                let mut sql = format!("SELECT {columns_sql} FROM {table}");
                
                // Add sorting if active (use original column names for sorting)
                if let Some(sort_clause) = self.state.get_sort_sql_clause(&column_names) {
                    sql.push(' ');
                    sql.push_str(&sort_clause);
                }
                
                // Add limit
                sql.push_str(" LIMIT 1000");

                match self.execute_query_direct(connection, &sql) {
                    Ok(data) => {
                        self.state.set_table_data(data);
                    }
                    Err(e) => {
                        self.state.show_error(format!("Failed to load table data: {e}"));
                    }
                }
            } else {
                self.state.show_error("No database connection available".to_string());
            }
        } else {
            self.state.show_error("No table selected for data loading".to_string());
        }
    }

    fn fetch_table_data_preserve_column(&mut self) {
        self.fetch_table_data_preserve_column_with_limit(true);
    }

    fn fetch_table_data_preserve_column_with_limit(&mut self, include_limit: bool) {
        if let (Some(_db), Some(table)) = (self.state.selected_database.clone(), self.state.selected_table.clone()) {
            if let Some(connection) = self.database_manager.get_current_connection() {
                // First get column names to enable sorting, filtering, and virtual ordering
                let column_names = match self.get_table_column_names(connection, &table) {
                    Ok(names) => names,
                    Err(e) => {
                        self.state.show_error(format!("Failed to get column names: {e}"));
                        return;
                    }
                };

                // Initialize column order if needed
                self.state.initialize_column_order(column_names.clone());
                
                // Get visible columns in virtual order
                let visible_column_names = self.state.get_visible_column_names();
                
                // Use visible columns directly (they are already in virtual order)
                let ordered_columns = visible_column_names;

                // Build base SQL query with virtual column ordering
                let columns_sql = ordered_columns.join(", ");
                let mut sql = format!("SELECT {columns_sql} FROM {table}");
                
                // Add filtering if active (use original column names for filtering)
                if let Some(filter_clause) = self.state.get_filter_sql_clause(&column_names) {
                    sql.push(' ');
                    sql.push_str(&filter_clause);
                }
                
                // Add sorting if active (use original column names for sorting)
                if let Some(sort_clause) = self.state.get_sort_sql_clause(&column_names) {
                    sql.push(' ');
                    sql.push_str(&sort_clause);
                }
                
                // Add limit conditionally
                if include_limit {
                    sql.push_str(" LIMIT 1000");
                }

                match self.execute_query_direct(connection, &sql) {
                    Ok(data) => {
                        self.state.update_table_data_preserve_column(data);
                    }
                    Err(e) => {
                        self.state.show_error(format!("Failed to load table data: {e}"));
                    }
                }
            } else {
                self.state.show_error("No database connection available".to_string());
            }
        } else {
            self.state.show_error("No table selected for data loading".to_string());
        }
    }

    fn get_table_column_names(&self, connection: &duckdb::Connection, table_name: &str) -> anyhow::Result<Vec<String>> {
        let sql = format!("SELECT * FROM {table_name} LIMIT 0");
        let mut stmt = connection.prepare(&sql)?;
        let _rows = stmt.query([])?;
        
        let column_count = stmt.column_count();
        let mut column_names = Vec::new();
        
        for i in 0..column_count {
            let column_name = stmt.column_name(i)
                .unwrap_or(&format!("column_{i}"))
                .to_string();
            column_names.push(column_name);
        }
        
        Ok(column_names)
    }

    fn get_table_schema(&self, connection: &duckdb::Connection, table_name: &str) -> anyhow::Result<crate::db::query::QueryResult> {
        let sql = format!("DESCRIBE {table_name}");
        self.execute_query_direct(connection, &sql)
    }

    fn get_table_statistics(&self, connection: &duckdb::Connection, table_name: &str) -> anyhow::Result<crate::db::query::QueryResult> {
        let sql = format!("SUMMARIZE {table_name}");
        self.execute_query_direct(connection, &sql)
    }

    fn execute_query_direct(&self, connection: &duckdb::Connection, sql: &str) -> anyhow::Result<crate::db::query::QueryResult> {
        let start_time = std::time::Instant::now();
        
        // Prepare statement and execute query
        let mut stmt = connection.prepare(sql)?;
        let mut rows = stmt.query([])?;
        
        // Get column count from the rows result
        let column_count = rows.as_ref().unwrap().column_count();
        
        // Get column names from the statement
        let mut columns = Vec::new();
        for i in 0..column_count {
            let column_name = rows.as_ref().unwrap().column_name(i)
                .unwrap_or(&format!("column_{i}"))
                .to_string();
            columns.push(column_name);
        }
        
        // Collect all rows
        let mut result_rows = Vec::new();
        while let Some(row) = rows.next()? {
            let mut row_data = Vec::new();
            for i in 0..column_count {
                // Convert each column value to string with safer error handling
                // Try f64 first to handle NaN values, then other types
                let value = match row.get::<_, f64>(i) {
                    Ok(v) => {
                        if v.is_nan() {
                            "NaN".to_string()
                        } else if v.is_infinite() {
                            if v.is_sign_positive() { "Infinity".to_string() } else { "-Infinity".to_string() }
                        } else {
                            v.to_string()
                        }
                    },
                    Err(_) => match row.get::<_, String>(i) {
                        Ok(v) => v,
                        Err(_) => match row.get::<_, i64>(i) {
                            Ok(v) => v.to_string(),
                            Err(_) => match row.get::<_, bool>(i) {
                                Ok(v) => v.to_string(),
                                Err(_) => {
                                    // Try to get as raw value or default to NULL
                                    match row.get_ref(i) {
                                        Ok(value_ref) => format!("{value_ref:?}"),
                                        Err(_) => "NULL".to_string(),
                                    }
                                }
                            }
                        }
                    }
                };
                row_data.push(value);
            }
            result_rows.push(row_data);
        }
        
        let execution_time = start_time.elapsed();
        
        Ok(crate::db::query::QueryResult {
            columns,
            row_count: result_rows.len(),
            rows: result_rows,
            execution_time_ms: execution_time.as_millis() as u64,
        })
    }

    fn handle_enter(&mut self) {
        // Handle modifying mode confirmation first
        if self.state.is_modifying {
            self.state.confirm_modifying();
            // No popup - will show in status bar instead
            return;
        }
        
        match self.state.active_panel {
            NavigationPanel::DatabaseList => {
                // Database panel Enter handling is now handled by state pattern
                // This fallback should not be reached when state pattern is active
            }
            NavigationPanel::MainContent => {
                // Toggle column expansion when viewing table data (but not in modifying mode)
                if self.state.table_data.is_some() && !self.state.is_modifying {
                    self.state.toggle_column_expansion();
                }
            }
            _ => {}
        }
    }

    fn start_table_creation(&mut self) {
        if self.database_manager.get_current_database().is_some() {
            self.state.start_table_creation();
        } else {
            self.state.show_error("Please connect to a database first".to_string());
        }
    }

    pub fn update_notifications(&mut self) {
        self.state.remove_expired_notifications();
        self.state.update_flash_timer();
        self.check_search_debounce();
    }

    fn show_help(&mut self) {
        self.state.show_info("Keys: Tab=Navigate | hjkl=Select | m=Reorder Mode | Enter=Confirm | i=Import/Inspect | o=Open | n=New | s=Save | d=Delete | ?=Help | q=Quit".to_string());
    }

    fn open_file_browser(&mut self) {
        match FileBrowser::new() {
            Ok(browser) => {
                self.file_browser = Some(browser);
                self.show_file_browser = true;
                self.state.show_info("File browser opened".to_string());
            }
            Err(e) => {
                self.state.show_error(format!("Error opening file browser: {e}"));
            }
        }
    }





    fn create_view_from_current_state(&mut self) {
        if let (Some(connection), Some(table_name)) = (
            self.database_manager.get_current_connection(),
            &self.state.selected_table
        ) {
            let view_name = self.state.new_view_name.trim().to_string();
            
            // Generate SQL from current view state
            if let Some(query_sql) = self.state.generate_view_sql(table_name) {
                let create_view_sql = format!("CREATE VIEW {view_name} AS {query_sql}");
                
                // Execute the CREATE VIEW statement
                match connection.execute(&create_view_sql, []) {
                    Ok(_) => {
                        self.state.show_success(format!("View '{view_name}' created successfully"));
                        
                        // Refresh the table list to show the new view
                        if let Some(current_db) = self.database_manager.get_current_database() {
                            let db_name = current_db.to_string();
                            if let Err(e) = self.database_manager.refresh_database(&db_name) {
                                self.action_logger.log_error(&format!("Failed to refresh table list: {e}"));
                            }
                        }
                    }
                    Err(e) => {
                        self.state.show_error(format!("Failed to create view: {e}"));
                        self.action_logger.log_error(&format!("CREATE VIEW failed: {e}"));
                    }
                }
            } else {
                self.state.show_error("Failed to generate SQL for current view state".to_string());
            }
        } else {
            self.state.show_error("No database connection or table selected".to_string());
        }
    }

    pub fn render(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(0),    // Main content
                Constraint::Length(3), // Status bar
            ])
            .split(f.area());

        // Header
        let header = Paragraph::new("🦆 Ducky - DuckDB TUI")
            .style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(header, chunks[0]);

        // Check for inspect mode - use full area for inspection
        if self.state.inspect_mode {
            if let Some(table) = self.state.selected_table.clone() {
                self.render_inspect_view(f, chunks[1], &table);
            }
        } else {
            // Normal mode: Main content area - New 2-panel layout
            let main_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(30), // Left sidebar
                    Constraint::Percentage(70), // Table viewer
                ])
                .split(chunks[1]);

            // Sync flash state before rendering to ensure current flash status
            self.sync_to_state_manager();
            
            // Left sidebar (combined database + table list)
            self.render_left_sidebar(f, main_chunks[0]);

            // Table viewer (renamed from main content)
            self.render_table_viewer(f, main_chunks[1]);
        }

        // Status bar
        self.render_status_bar(f, chunks[2]);

        // Render notifications
        self.render_notifications(f, f.area());

        // Render file browser popup if shown
        if self.show_file_browser {
            if let Some(ref browser) = self.file_browser {
                render_file_browser_popup(f, f.area(), browser);
            }
        }

        // Database dropdown overlay is now handled by state pattern


        // Render view name input popup
        if self.state.is_entering_view_name {
            self.render_view_name_input(f, f.area());
        }

        
        // Render state pattern modals
        if self.state_manager.has_active_modal() {
            self.state_manager.render_top_modal(f, f.area(), &self.database_manager);
        }
    }

    fn render_left_sidebar(&self, f: &mut Frame, area: Rect) {
        // Split sidebar into database dropdown area and table list
        let sidebar_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Database dropdown area
                Constraint::Min(0),    // Table list area
            ])
            .split(area);

        // Render database select using state pattern
        self.state_manager.render_database_select(f, sidebar_chunks[0], &self.database_manager);
        
        // Render table list in bottom area using state pattern
        self.state_manager.render_table_list(f, sidebar_chunks[1], &self.database_manager);
    }


    fn get_panel_border_style(&self, panel: NavigationPanel) -> Style {
        let is_active = self.state.active_panel == panel;
        if is_active {
            if self.state.is_panel_flashing() {
                // Flash effect: bright cyan
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                // Normal active: green
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            }
        } else {
            // Inactive: default
            Style::default()
        }
    }

    fn render_table_viewer(&mut self, f: &mut Frame, area: Rect) {
        // Check if we're in inspect mode and have a selected table
        if self.state.inspect_mode {
            if let Some(table) = self.state.selected_table.clone() {
                self.render_inspect_view(f, area, &table);
                return;
            }
        }

        // Cache area height for navigation calculations
        self.state.last_table_area_height = area.height;
        let border_style = self.get_panel_border_style(NavigationPanel::MainContent);

        let (content, title): (String, String) = if self.state.current_state == AppState::ImportWizard && self.state.is_creating_table {
            match self.state.table_creation_step {
                TableCreationStep::EnteringTableName => {
                    let content = format!(
                        "Create New Table\n\nTable name: {}\n\nType the table name and press Enter to continue\nPress Esc to cancel",
                        if self.state.new_table_name.is_empty() { 
                            "_" 
                        } else { 
                            &self.state.new_table_name 
                        }
                    );
                    (content, "Import Wizard - Table Name".to_string())
                }
                TableCreationStep::SelectingFile => {
                    let content = format!(
                        "Create New Table: '{}'\n\nSelect a file to import data from:\n• CSV files (.csv)\n• JSON files (.json)\n• Parquet files (.parquet)\n\nPress Esc to cancel",
                        self.state.new_table_name
                    );
                    (content, "Import Wizard - File Selection".to_string())
                }
                TableCreationStep::ImportingData => {
                    let content = format!(
                        "Create New Table: '{}'\n\nImporting data...\n\nPlease wait while the data is being imported.",
                        self.state.new_table_name
                    );
                    (content, "Import Wizard - Importing".to_string())
                }
            }
        } else if let (Some(db), Some(table)) = (&self.state.selected_database, &self.state.selected_table) {
            if let Some(ref data) = self.state.table_data {
                // Render table using ratatui Table widget instead of string content
                // Build table title with sort status
                let sort_info = if !self.state.sort_columns.is_empty() {
                    let mut sort_parts = Vec::new();
                    for sort_spec in &self.state.sort_columns {
                        let direction = match sort_spec.direction {
                            SortDirection::Ascending => "ASC",
                            SortDirection::Descending => "DESC",
                        };
                        sort_parts.push(format!("{} {}", sort_spec.column_name, direction));
                    }
                    
                    if !sort_parts.is_empty() {
                        format!(" - Sorted by: {}", sort_parts.join(", "))
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                };
                
                let title = format!("Table: {} ({} rows){}", table, data.row_count, sort_info);
                self.render_table_widget(f, area, data, &title);
                return; // Early return since we handled rendering directly
            } else {
                let content = format!(
                    "Database: {db}\nTable: {table}\n\nLoading table data...\n\nPress Enter on table to load data or wait for auto-load"
                );
                (content, "Table Viewer".to_string())
            }
        } else {
            let debug_info = format!(
                "Database: {:?}\nTable: {:?}", 
                self.state.selected_database, 
                self.state.selected_table
            );
            let content = format!(
                "Select a database and table to view data\n\nNavigation:\n• Use Tab/Shift+Tab to switch panels\n• Use ↑↓ to navigate lists\n• Press Enter to select items\n• Press i to import data\n• Press h for help\n• Press q or Esc to quit\n\nDebug:\n{}", 
                debug_info
            );
            (content, "Main Content [3]".to_string())
        };

        let paragraph = Paragraph::new(content)
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(border_style),
            )
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(Color::White));

        f.render_widget(paragraph, area);
    }

    fn render_inspect_view(&mut self, f: &mut Frame, area: Rect, table_name: &str) {
        // Get schema and statistics data
        let (schema_data, stats_data) = if let Some(connection) = self.database_manager.get_current_connection() {
            let schema = self.get_table_schema(connection, table_name)
                .unwrap_or_else(|_| crate::db::query::QueryResult::new());
            let stats = self.get_table_statistics(connection, table_name)
                .unwrap_or_else(|_| crate::db::query::QueryResult::new());
            (schema, stats)
        } else {
            (crate::db::query::QueryResult::new(), crate::db::query::QueryResult::new())
        };

        // Split area into two sections: schema on top, statistics on bottom
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(50), // Schema section
                Constraint::Percentage(50), // Statistics section
            ])
            .split(area);

        // Render schema section
        self.render_schema_section(f, chunks[0], table_name, &schema_data);
        
        // Render statistics section
        self.render_statistics_section(f, chunks[1], table_name, &stats_data);
    }

    fn render_schema_section(&self, f: &mut Frame, area: Rect, table_name: &str, schema_data: &crate::db::query::QueryResult) {
        // Determine if this section is active and style accordingly
        let is_active = matches!(self.state.inspect_active_section, crate::app::state::InspectSection::Schema);
        let border_style = if is_active {
            Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        
        // Create title with active indicator
        let title = if is_active {
            if self.state.is_modifying {
                format!("► Columns: {} (MODIFY MODE - j/k to move, J/K for extremes, o to hide/show, Enter to confirm, Esc to cancel)", table_name)
            } else {
                format!("► Columns: {} (Tab to switch, j/k to scroll, m to modify)", table_name)
            }
        } else {
            format!("Columns: {}", table_name)
        };

        if schema_data.rows.is_empty() {
            // No schema data available
            let content = "No column information available for this table.";
            let paragraph = Paragraph::new(content)
                .block(
                    Block::default()
                        .title(title)
                        .borders(Borders::ALL)
                        .border_style(border_style),
                )
                .wrap(Wrap { trim: true })
                .style(Style::default().fg(Color::White));
            f.render_widget(paragraph, area);
            return;
        }

        // Build table rows from schema data, respecting virtual column order
        let virtual_order = self.state.get_virtual_column_order();
        let original_columns = self.state.get_original_column_names();
        
        // Reorder schema data to match virtual column order
        let mut ordered_schema_rows = Vec::new();
        for virtual_col_name in &virtual_order {
            // Find the index of this column in the original schema
            if let Some(original_idx) = original_columns.iter().position(|name| name == virtual_col_name) {
                if original_idx < schema_data.rows.len() {
                    ordered_schema_rows.push(schema_data.rows[original_idx].clone());
                }
            }
        }
        
        // Create header in virtual order (add extra columns for enhanced display)
        let header = vec![
            Cell::from("Column Name"),
            Cell::from("Data Type"),
            Cell::from("Nullable"),
            Cell::from("Sort Order"),
            Cell::from("Sort Direction"),
            Cell::from("Hidden"),
        ];
        
        // Apply scrolling: skip rows based on scroll position
        let scroll_offset = self.state.inspect_schema_scroll_y;
        let rows: Vec<Row> = ordered_schema_rows.iter()
            .enumerate()
            .skip(scroll_offset)
            .map(|(row_idx, row)| {
                // Get the virtual column index
                let virtual_col_idx = scroll_offset + row_idx;
                
                // Get the column name from virtual order
                let column_name = if virtual_col_idx < virtual_order.len() {
                    &virtual_order[virtual_col_idx]
                } else {
                    return Row::new(vec![Cell::from("ERROR")]);
                };
                
                // Check if this column is hidden by name
                let is_hidden = self.state.is_column_hidden_by_name(column_name);
                
                // Get sort information
                let sort_info = self.state.sort_columns.iter()
                    .position(|spec| spec.column_name == *column_name)
                    .map(|pos| (pos + 1, &self.state.sort_columns[pos].direction));
                
                let (sort_order, sort_direction) = match sort_info {
                    Some((order, direction)) => {
                        let dir_str = match direction {
                            SortDirection::Ascending => "ASC",
                            SortDirection::Descending => "DESC",
                        };
                        (order.to_string(), dir_str.to_string())
                    },
                    None => ("".to_string(), "".to_string()),
                };
                
                // Build enhanced row with additional columns
                let base_style = if is_hidden {
                    Style::default().fg(Color::DarkGray)
                } else {
                    Style::default()
                };
                
                let cells = vec![
                    Cell::from(row.first().unwrap_or(&"?".to_string()).clone()).style(base_style),  // Column Name
                    Cell::from(row.get(1).unwrap_or(&"?".to_string()).clone()).style(base_style),  // Data Type
                    Cell::from(row.get(2).unwrap_or(&"?".to_string()).clone()).style(base_style),  // Nullable
                    Cell::from(sort_order.clone()).style(base_style),                              // Sort Order
                    Cell::from(sort_direction.clone()).style(base_style),                          // Sort Direction
                    Cell::from(if is_hidden { "YES" } else { "NO" }).style(base_style),            // Hidden
                ];
                
                Row::new(cells)
            }).collect();

        // Use fixed column widths for the enhanced display (6 columns)
        let constraints: Vec<Constraint> = vec![
            Constraint::Length(15),  // Column Name
            Constraint::Length(12),  // Data Type
            Constraint::Length(8),   // Nullable
            Constraint::Length(10),  // Sort Order
            Constraint::Length(12),  // Sort Direction
            Constraint::Length(6),   // Hidden
        ];

        let table = Table::new(rows, constraints)
            .header(Row::new(header).style(Style::default().add_modifier(Modifier::BOLD)))
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(border_style),
            )
            .highlight_style(if is_active {
                Style::default().bg(Color::Gray).fg(Color::Black).add_modifier(Modifier::BOLD)
            } else {
                Style::default().bg(Color::DarkGray)
            })
            .style(Style::default().fg(Color::White));

        // Create table state for selection
        let mut table_state = TableState::default();
        if is_active {
            // Show selection only when this section is active
            let selected_index = self.state.inspect_selected_row.saturating_sub(scroll_offset);
            table_state.select(Some(selected_index));
        }

        f.render_stateful_widget(table, area, &mut table_state);
    }

    fn render_statistics_section(&self, f: &mut Frame, area: Rect, table_name: &str, stats_data: &crate::db::query::QueryResult) {
        // Determine if this section is active and style accordingly
        let is_active = matches!(self.state.inspect_active_section, crate::app::state::InspectSection::Statistics);
        let border_style = if is_active {
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        
        // Create title with active indicator
        let title = if is_active {
            format!("► Statistics: {table_name} (Tab to switch, ↑↓ to scroll, Esc to exit)")
        } else {
            format!("Statistics: {table_name} (Press Esc to exit inspect mode)")
        };

        if stats_data.rows.is_empty() {
            // No statistics data available
            let content = "No statistics available for this table.";
            let paragraph = Paragraph::new(content)
                .block(
                    Block::default()
                        .title(title)
                        .borders(Borders::ALL)
                        .border_style(border_style),
                )
                .wrap(Wrap { trim: true })
                .style(Style::default().fg(Color::White));
            f.render_widget(paragraph, area);
            return;
        }

        // Build table rows from statistics data
        let header = stats_data.columns.iter().map(|col| Cell::from(col.as_str())).collect::<Vec<_>>();
        
        // Apply scrolling: skip rows based on scroll position
        let scroll_offset = self.state.inspect_stats_scroll_y;
        let rows: Vec<Row> = stats_data.rows.iter()
            .skip(scroll_offset)
            .map(|row| {
                let cells: Vec<Cell> = row.iter().map(|cell| Cell::from(cell.as_str())).collect();
                Row::new(cells)
            }).collect();

        // Calculate column widths based on content
        let mut column_widths = vec![0; stats_data.columns.len()];
        
        // Check header widths
        for (i, col) in stats_data.columns.iter().enumerate() {
            column_widths[i] = col.len().max(column_widths[i]);
        }
        
        // Check data widths
        for row in &stats_data.rows {
            for (i, cell) in row.iter().enumerate() {
                if i < column_widths.len() {
                    column_widths[i] = cell.len().max(column_widths[i]);
                }
            }
        }

        // Convert to constraints with minimum and maximum widths
        let constraints: Vec<Constraint> = column_widths.iter().map(|&width| {
            let min_width = 8; // Minimum column width
            let max_width = 20; // Slightly smaller max for statistics to fit more columns
            let adjusted_width = width.max(min_width).min(max_width);
            Constraint::Length(adjusted_width as u16)
        }).collect();

        let table = Table::new(rows, constraints)
            .header(Row::new(header).style(Style::default().add_modifier(Modifier::BOLD)))
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(border_style),
            )
            .highlight_style(Style::default().bg(Color::DarkGray))
            .style(Style::default().fg(Color::White));

        f.render_widget(table, area);
    }

    fn calculate_inspect_scroll_bounds(&self) -> (usize, usize) {
        // Calculate scrolling bounds for inspect mode
        // Each section gets 50% of the main content area height
        let section_height = (self.state.last_table_area_height / 2).saturating_sub(3) as usize; // Subtract for borders
        let visible_rows = section_height.max(5); // Minimum 5 visible rows
        
        // Calculate max rows based on active section
        let max_rows = match self.state.inspect_active_section {
            crate::app::state::InspectSection::Schema => {
                // For schema, estimate based on typical column count
                // Most tables have 3-20 columns, so max_rows would be that count
                20 // Conservative estimate - in practice this will be determined by actual data
            }
            crate::app::state::InspectSection::Statistics => {
                // For statistics, DuckDB SUMMARIZE typically returns one row per column
                // So this is similar to schema but may be longer with statistics per column
                50 // Conservative estimate for statistics rows
            }
        };
        
        (max_rows, visible_rows)
    }

    fn render_status_bar(&self, f: &mut Frame, area: Rect) {
        if self.state.is_searching {
            // Search mode: use entire status bar for search input
            self.render_search_input(f, area);
        } else {
            // Normal mode: split status bar into left and right sections
            let status_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(60), // Left section - general status
                    Constraint::Percentage(40), // Right section - table viewer status
                ])
                .split(area);

            // Render left status section (general status)
            self.render_left_status(f, status_chunks[0]);
            
            // Render right status section (table viewer status)
            self.render_table_status(f, status_chunks[1]);
        }
    }

    fn render_search_input(&self, f: &mut Frame, area: Rect) {
        // Determine border color based on syntax validity
        let border_color = if self.state.search_syntax_valid {
            Color::Green // Green border for valid syntax
        } else {
            Color::Red // Red border for invalid syntax
        };

        // Get column name for context
        let column_context = if let (Some(column_index), Some(ref data)) = (self.state.search_column, &self.state.table_data) {
            if column_index < data.columns.len() {
                format!(" [{}]", data.columns[column_index])
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        // Create search input display with cursor
        let search_display = format!("Filter{}: {}_", column_context, self.state.search_text);
        
        // Calculate available width (subtract borders and padding)
        let available_width = area.width.saturating_sub(4) as usize; // 2 for borders + 2 for padding
        let display_text = truncate_text(&search_display, available_width);

        let search_input = Paragraph::new(display_text)
            .block(
                Block::default()
                    .title("Search/Filter")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(border_color)),
            )
            .style(Style::default().fg(Color::White));

        f.render_widget(search_input, area);
    }

    fn check_search_debounce(&mut self) {
        if self.state.is_searching && self.state.search_syntax_valid && self.state.should_debounce_update() {
            // Reset debounce timer to prevent multiple updates
            self.state.search_debounce_timer = None;
            
            // Trigger live update if search text is not empty
            if !self.state.search_text.trim().is_empty() {
                // Create a temporary filter for live preview
                if let Some(column_index) = self.state.search_column {
                    // Store current filters state
                    let saved_filters = self.state.column_filters.clone();
                    
                    // Temporarily add the search text as a filter
                    if let Some(column_name) = self.state.get_column_name_by_index(column_index) {
                        self.state.column_filters.insert(column_name, self.state.search_text.trim().to_string());
                    }
                    
                    // Update the view with limit for performance during preview
                    self.fetch_table_data_preserve_column_with_limit(true);
                    
                    // Restore the saved filters (don't persist the preview)
                    self.state.column_filters = saved_filters;
                }
            }
        }
    }

    fn validate_search_syntax(&mut self) {
        if !self.state.is_searching {
            return;
        }

        // If search text is empty, consider it valid
        if self.state.search_text.trim().is_empty() {
            self.state.search_syntax_valid = true;
            return;
        }

        // Try to validate by building a test query
        if let (Some(column_index), Some(ref table), Some(ref data)) = 
            (self.state.search_column, &self.state.selected_table, &self.state.table_data) {
            
            if column_index < data.columns.len() {
                let column_name = &data.columns[column_index];
                let test_sql = format!(
                    "SELECT COUNT(*) FROM {} WHERE {} {}",
                    table, column_name, self.state.search_text.trim()
                );

                // Try to prepare the statement to validate syntax
                if let Some(connection) = self.database_manager.get_current_connection() {
                    match connection.prepare(&test_sql) {
                        Ok(_) => {
                            self.state.search_syntax_valid = true;
                        }
                        Err(_) => {
                            self.state.search_syntax_valid = false;
                        }
                    }
                } else {
                    // No connection available, assume valid for now
                    self.state.search_syntax_valid = true;
                }
            }
        }
    }

    fn render_left_status(&self, f: &mut Frame, area: Rect) {
        let border_style = self.get_panel_border_style(NavigationPanel::StatusBar);

        // Calculate available width (subtract borders and padding)
        let available_width = area.width.saturating_sub(4) as usize; // 2 for borders + 2 for padding
        let status_text = truncate_text(&self.state.get_status_display(), available_width);

        let status = Paragraph::new(status_text)
            .block(
                Block::default()
                    .title("Status")
                    .borders(Borders::ALL)
                    .border_style(border_style),
            )
            .style(Style::default().fg(Color::Cyan))
            .wrap(Wrap { trim: false }); // Disable wrapping since we're truncating

        f.render_widget(status, area);
    }

    fn render_table_status(&self, f: &mut Frame, area: Rect) {
        let border_style = self.get_panel_border_style(NavigationPanel::MainContent);

        // Generate table-specific status info
        let table_status = if let Some(ref data) = self.state.table_data {
            format!(
                "Row: {} of {} | Col: {} of {} | ←→↑↓ navigate",
                self.state.selected_row + 1,
                data.rows.len(),
                self.state.get_selected_column_index().map(|i| i + 1).unwrap_or(1),
                data.columns.len()
            )
        } else {
            "No table data".to_string()
        };

        // Calculate available width (subtract borders and padding)
        let available_width = area.width.saturating_sub(4) as usize; // 2 for borders + 2 for padding
        let truncated_status = truncate_text(&table_status, available_width);

        let status = Paragraph::new(truncated_status)
            .block(
                Block::default()
                    .title("Table Info")
                    .borders(Borders::ALL)
                    .border_style(border_style),
            )
            .style(Style::default().fg(Color::Yellow))
            .wrap(Wrap { trim: false }); // Disable wrapping since we're truncating

        f.render_widget(status, area);
    }

    fn render_notifications(&self, f: &mut Frame, area: Rect) {
        if self.state.notifications.is_empty() {
            return;
        }

        // Calculate notification area (centered, 60% width, near top)
        let notification_width = (area.width as f32 * 0.6) as u16;
        let notification_height = 3;
        let x = (area.width.saturating_sub(notification_width)) / 2;
        let y = 5; // Position near top

        // Only show the most recent notification to avoid overlap
        if let Some(notification) = self.state.notifications.last() {
            let notification_area = Rect {
                x,
                y,
                width: notification_width,
                height: notification_height,
            };

            // Choose colors based on notification type - simple black background with colored borders
            let (border_color, icon) = match notification.notification_type {
                crate::app::state::NotificationType::Success => (Color::Green, "✅"),
                crate::app::state::NotificationType::Error => (Color::Red, "❌"),
                crate::app::state::NotificationType::Info => (Color::Blue, "ℹ️"),
            };

            // Truncate notification text to fit
            let available_width = notification_width.saturating_sub(6) as usize; // Account for borders + icon
            let display_text = format!("{} {}", icon, truncate_text(&notification.message, available_width));

            let notification_widget = Paragraph::new(display_text)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default()
                            .fg(border_color)
                            .add_modifier(Modifier::BOLD))
                )
                .style(Style::default()
                    .fg(Color::White)
                    .bg(Color::Black)
                    .add_modifier(Modifier::BOLD))
                .alignment(Alignment::Center);

            // Render the notification with opaque black background
            f.render_widget(notification_widget, notification_area);
        }
    }




    fn render_view_name_input(&self, f: &mut Frame, area: Rect) {
        // Create centered popup
        let popup_width = 60;
        let popup_height = 8;
        let x = (area.width.saturating_sub(popup_width)) / 2;
        let y = (area.height.saturating_sub(popup_height)) / 2;
        let popup_area = Rect {
            x,
            y,
            width: popup_width,
            height: popup_height,
        };

        let unknown_table = "unknown".to_string();
        let current_table = self.state.selected_table.as_ref().unwrap_or(&unknown_table);
        let display_view_name = if self.state.new_view_name.is_empty() {
            "_"
        } else {
            &self.state.new_view_name
        };

        // Generate a preview of the SQL
        let sql_preview = if let Some(sql) = self.state.generate_view_sql(current_table) {
            let truncated = if sql.len() > 100 {
                format!("{}...", &sql[..97])
            } else {
                sql
            };
            format!("\nSQL: {truncated}")
        } else {
            "\nSQL: SELECT * FROM table".to_string()
        };

        let content = format!(
            "Save View from Table '{}'\n\nView name: {}{}\n\nPress Enter to create view, Esc to cancel",
            current_table,
            display_view_name,
            sql_preview
        );

        let popup = Paragraph::new(content)
            .block(
                Block::default()
                    .title("Create View")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD)),
            )
            .style(Style::default().fg(Color::White).bg(Color::Black).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center);

        f.render_widget(popup, popup_area);
    }

}

/// Truncate text to fit within a specific width, adding "..." if needed
/// This function properly handles Unicode character boundaries
fn truncate_text(text: &str, max_width: usize) -> String {
    if max_width <= 3 {
        return "...".to_string();
    }
    
    // Count characters, not bytes, for proper Unicode handling
    let char_count = text.chars().count();
    if char_count <= max_width {
        text.to_string()
    } else {
        // Take only the characters that fit, respecting Unicode boundaries
        let truncated: String = text.chars()
            .take(max_width.saturating_sub(3))
            .collect();
        format!("{truncated}...")
    }
}

/// Wrap text to fit within a specific width, returning wrapped lines
/// This function properly handles Unicode character boundaries
fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 {
        return vec![];
    }
    
    let char_count = text.chars().count();
    if char_count <= max_width {
        return vec![text.to_string()];
    }
    
    let mut lines = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    
    let mut start = 0;
    while start < chars.len() {
        let end = (start + max_width).min(chars.len());
        let line: String = chars[start..end].iter().collect();
        lines.push(line);
        start = end;
    }
    
    lines
}
