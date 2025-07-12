use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap, Table, Row, Cell},
    Frame,
};

use crate::actions::ActionLogger;
use crate::app::state::{ApplicationState, NavigationPanel, AppState, TableCreationStep};
use crate::db::DatabaseManager;
use crate::workflows::DatabaseWorkflows;

mod file_browser;
use file_browser::{render_file_browser_popup, FileBrowser, detect_file_type, FileType};

pub struct App {
    state: ApplicationState,
    database_manager: DatabaseManager,
    selected_db_index: usize,
    selected_table_index: usize,
    file_browser: Option<FileBrowser>,
    show_file_browser: bool,
    action_logger: ActionLogger,
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
            selected_db_index: 0,
            selected_table_index: 0,
            file_browser: None,
            show_file_browser: false,
            action_logger,
        };
        
        app.sync_selected_db_index();
        app.sync_selected_table_index();
        
        // If we loaded a database via CLI, update the application state to reflect the selection
        if database_path.is_some() {
            if let Some(current_db) = app.database_manager.get_current_database() {
                app.state.select_database(current_db.to_string());
            }
        }
        
        app
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
                                        self.sync_selected_db_index();
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
                                                    self.sync_selected_table_index();
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

        // Handle database name input
        if self.state.is_entering_database_name {
            match key.code {
                KeyCode::Esc => {
                    self.state.cancel_database_name_input();
                }
                KeyCode::Enter => {
                    if !self.state.new_database_name.trim().is_empty() {
                        self.create_database_with_name();
                        self.state.cancel_database_name_input();
                    }
                }
                KeyCode::Backspace => {
                    self.state.remove_char_from_database_name();
                }
                KeyCode::Char(c) => {
                    self.state.add_char_to_database_name(c);
                }
                _ => {}
            }
            return;
        }

        // Handle save filename input
        if self.state.is_entering_save_filename {
            match key.code {
                KeyCode::Esc => {
                    self.state.cancel_save_filename_input();
                }
                KeyCode::Enter => {
                    if !self.state.save_filename.trim().is_empty() {
                        self.save_current_database_to_file();
                        self.state.cancel_save_filename_input();
                    }
                }
                KeyCode::Backspace => {
                    self.state.remove_char_from_save_filename();
                }
                KeyCode::Char(c) => {
                    self.state.add_char_to_save_filename(c);
                }
                _ => {}
            }
            return;
        }

        // Handle delete confirmation
        if self.state.is_delete_confirmation_active() {
            match key.code {
                KeyCode::Esc => {
                    self.state.cancel_delete_confirmation();
                }
                KeyCode::Char('d') => {
                    self.confirm_delete();
                    self.state.cancel_delete_confirmation();
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

        // Normal key handling
        match key.code {
            KeyCode::Tab => {
                if self.state.database_dropdown_expanded {
                    // Close dropdown without making changes when Tab is pressed
                    self.state.collapse_database_dropdown();
                    self.state.set_dropdown_to_current_database(self.selected_db_index);
                } else {
                    self.state.next_panel();
                }
            }
            KeyCode::BackTab => self.state.prev_panel(),
            KeyCode::Esc => {
                if self.state.database_dropdown_expanded {
                    // Close dropdown without making changes when Escape is pressed
                    self.state.collapse_database_dropdown();
                    self.state.set_dropdown_to_current_database(self.selected_db_index);
                }
                // Note: Could add other escape behaviors here in the future
            }
            KeyCode::Up => self.handle_up(),
            KeyCode::Down => self.handle_down(),
            KeyCode::Left => self.handle_left(),
            KeyCode::Right => self.handle_right(),
            KeyCode::Enter => self.handle_enter(),
            KeyCode::Char('h') => self.show_help(),
            KeyCode::Char('i') => self.start_table_creation(),
            KeyCode::Char('o') => self.open_file_browser(),
            KeyCode::Char('n') => {
                // Start database name input
                self.state.start_database_name_input();
            }
            KeyCode::Char('s') => {
                // Start save database to file
                if self.database_manager.get_current_database().is_some() {
                    self.state.start_save_filename_input();
                } else {
                    self.state.show_error("No database selected to save".to_string());
                }
            }
            KeyCode::Char('d') => {
                // Start delete confirmation for current selection
                self.start_delete_confirmation();
            }
            KeyCode::Char('D') => {
                // Immediate delete without confirmation
                self.immediate_delete();
            }
            KeyCode::Char('q') => {
                // Quit handled by main loop
            }
            KeyCode::Char('x') => {
                // Disconnect current database (moved from 'd')
                if self.database_manager.get_current_database().is_some() {
                    let mut workflows = DatabaseWorkflows::new(
                        &mut self.database_manager,
                        &mut self.action_logger,
                        &mut self.state,
                    );
                    let _ = workflows.disconnect_current_database();
                    self.sync_selected_db_index();
                    self.selected_table_index = 0;
                }
            }
            KeyCode::Char('1') => self.state.set_left_panel(NavigationPanel::DatabaseList),
            KeyCode::Char('2') => self.state.set_left_panel(NavigationPanel::TableList),
            KeyCode::Char('3') => self.state.set_active_panel(NavigationPanel::MainContent),
            _ => {}
        }
    }

    fn handle_up(&mut self) {
        match self.state.active_panel {
            NavigationPanel::DatabaseList => {
                let databases = self.database_manager.get_databases();
                if !self.state.database_dropdown_expanded {
                    // Expand dropdown when first pressing up/down
                    self.state.expand_database_dropdown(databases.len());
                    self.state.set_dropdown_to_current_database(self.selected_db_index);
                } else {
                    // Navigate within dropdown
                    self.state.dropdown_move_up();
                }
            }
            NavigationPanel::TableList => {
                if self.selected_table_index > 0 {
                    self.selected_table_index -= 1;
                }
            }
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

    fn handle_down(&mut self) {
        let databases = self.database_manager.get_databases();
        let current_tables = self.get_current_tables();

        match self.state.active_panel {
            NavigationPanel::DatabaseList => {
                if !self.state.database_dropdown_expanded {
                    // Expand dropdown when first pressing up/down
                    self.state.expand_database_dropdown(databases.len());
                    self.state.set_dropdown_to_current_database(self.selected_db_index);
                } else {
                    // Navigate within dropdown
                    self.state.dropdown_move_down(databases.len());
                }
            }
            NavigationPanel::TableList => {
                if self.selected_table_index < current_tables.len().saturating_sub(1) {
                    self.selected_table_index += 1;
                }
            }
            NavigationPanel::MainContent => {
                // Move selected row down
                if let Some(ref data) = self.state.table_data {
                    // Calculate visible rows to pass to method
                    let visible_rows = 10; // Approximate - could be calculated from area
                    self.state.move_selected_down(data.rows.len(), visible_rows);
                }
            }
            _ => {}
        }
    }

    fn handle_left(&mut self) {
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
                self.state.collapse_database_dropdown(); // Close dropdown if open
            }
            _ => {}
        }
    }

    fn handle_right(&mut self) {
        match self.state.active_panel {
            NavigationPanel::DatabaseList => {
                // From database widget, go to tables widget
                self.state.set_left_panel(NavigationPanel::TableList);
                self.state.collapse_database_dropdown(); // Close dropdown if open
            }
            NavigationPanel::MainContent => {
                // Move selected column right
                if let Some(ref data) = self.state.table_data {
                    // Calculate visible columns to pass to method
                    let visible_cols = 5; // Approximate - could be calculated from area
                    self.state.move_selected_right(data.columns.len(), visible_cols);
                }
            }
            NavigationPanel::TableList => {
                // From table list, go to main content (table viewer)
                self.state.set_active_panel(NavigationPanel::MainContent);
            }
            _ => {}
        }
    }

    fn get_current_tables(&self) -> Vec<String> {
        if let Some(current_db) = self.database_manager.get_current_database() {
            if let Some(db_info) = self.database_manager.get_database_info(current_db) {
                return db_info.tables.iter().map(|t| t.name.clone()).collect();
            }
        }
        Vec::new()
    }

    fn sync_selected_db_index(&mut self) {
        if let Some(current_db) = self.database_manager.get_current_database() {
            let databases = self.database_manager.get_databases();
            if let Some(index) = databases.iter().position(|db| db.name == current_db) {
                self.selected_db_index = index;
            }
        }
    }

    fn sync_selected_table_index(&mut self) {
        if let Some(current_table) = &self.state.selected_table {
            let current_tables = self.get_current_tables();
            if let Some(index) = current_tables.iter().position(|table| table == current_table) {
                self.selected_table_index = index;
            }
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
        
        for (i, col_name) in data.columns.iter().enumerate() {
            let header_width = col_name.chars().count();
            let mut max_data_width = 0;
            
            // Calculate max data width for visible rows
            if self.state.is_column_expanded(i) {
                // For expanded columns, check all visible rows for more accurate width
                for row in data.rows[start_row..end_row].iter() {
                    if let Some(cell) = row.get(i) {
                        max_data_width = max_data_width.max(cell.chars().count());
                    }
                }
            } else {
                // For normal columns, sample first 10 rows for performance
                for row in data.rows.iter().take(10) {
                    if let Some(cell) = row.get(i) {
                        max_data_width = max_data_width.max(cell.chars().count());
                    }
                }
            }
            
            let col_width = if self.state.is_column_expanded(i) {
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

        // Create header with styling for selected column
        let header_cells: Vec<Cell> = visible_cols.iter().map(|&col_idx| {
            let cell_content = &data.columns[col_idx];
            let col_width = column_widths.get(col_idx).unwrap_or(&min_col_width);
            let text = if self.state.is_column_expanded(col_idx) {
                // For expanded columns, show full header without truncation
                cell_content.clone()
            } else {
                truncate_text(cell_content, *col_width)
            };
            
            if col_idx == self.state.selected_col {
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
                cell_lines.push(lines);
            }
            
            // Create multiple rows if any cell has wrapped content
            for line_idx in 0..max_lines {
                let cells: Vec<Cell> = visible_cols.iter().enumerate().map(|(visible_idx, &col_idx)| {
                    let line_text = cell_lines.get(visible_idx)
                        .and_then(|lines| lines.get(line_idx))
                        .unwrap_or(&String::new())
                        .clone();
                    
                    // Check if this is the current cell (intersection of selected row and column)
                    let is_current_cell = is_selected_row && col_idx == self.state.selected_col;
                    
                    if is_current_cell {
                        // Highlight current cell with light gray background and inverted text for readability
                        Cell::from(line_text).style(Style::default().bg(Color::Gray).fg(Color::Black).add_modifier(Modifier::BOLD))
                    } else if is_selected_row {
                        // Bold selected row
                        Cell::from(line_text).style(Style::default().add_modifier(Modifier::BOLD))
                    } else if col_idx == self.state.selected_col {
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


    fn fetch_table_data(&mut self) {
        if let (Some(_db), Some(table)) = (&self.state.selected_database, &self.state.selected_table) {
            if let Some(connection) = self.database_manager.get_current_connection() {
                let sql = format!("SELECT * FROM {table} LIMIT 1000");
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
        match self.state.active_panel {
            NavigationPanel::DatabaseList => {
                if self.state.database_dropdown_expanded {
                    // Select database from dropdown
                    let db_name = {
                        let databases = self.database_manager.get_databases();
                        databases
                            .get(self.state.dropdown_selected_index)
                            .map(|db| db.name.clone())
                    };

                    if let Some(db_name) = db_name {
                        let mut workflows = DatabaseWorkflows::new(
                            &mut self.database_manager,
                            &mut self.action_logger,
                            &mut self.state,
                        );
                        let _ = workflows.select_database(db_name);
                        self.sync_selected_db_index(); // Sync the index after selection
                        self.selected_table_index = 0; // Reset table selection
                    }
                    
                    // Close dropdown after selection
                    self.state.collapse_database_dropdown();
                } else {
                    // Expand dropdown if not already expanded
                    let databases = self.database_manager.get_databases();
                    self.state.expand_database_dropdown(databases.len());
                    self.state.set_dropdown_to_current_database(self.selected_db_index);
                }
            }
            NavigationPanel::TableList => {
                let current_tables = self.get_current_tables();
                if let Some(table_name) = current_tables.get(self.selected_table_index) {
                    let mut workflows = DatabaseWorkflows::new(
                        &mut self.database_manager,
                        &mut self.action_logger,
                        &mut self.state,
                    );
                    let _ = workflows.select_table(table_name.clone());
                    self.sync_selected_table_index();
                    self.fetch_table_data();
                }
            }
            NavigationPanel::MainContent => {
                // Toggle column expansion when viewing table data
                if self.state.table_data.is_some() {
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
    }

    fn show_help(&mut self) {
        self.state.show_info("Keys: Tab/Shift+Tab=Navigate | ↑↓=Select | Enter=Confirm | i=Import | o=Open | n=New DB | s=Save DB | d=Delete | D=Delete Now | x=Disconnect | q=Quit".to_string());
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

    fn create_database_with_name(&mut self) {
        let db_name = self.state.new_database_name.clone();
        
        // Create in-memory database with custom name
        if let Err(e) = self.database_manager.add_database(db_name.clone(), ":memory:".to_string()) {
            self.state.show_error(format!("Failed to create database: {e}"));
        } else {
            let mut workflows = DatabaseWorkflows::new(
                &mut self.database_manager,
                &mut self.action_logger,
                &mut self.state,
            );
            let _ = workflows.select_database(db_name.clone());
            self.sync_selected_db_index();
            self.selected_table_index = 0;
            self.state.show_success(format!("Created database '{db_name}'"));
        }
    }

    fn start_delete_confirmation(&mut self) {
        match self.state.active_panel {
            NavigationPanel::DatabaseList => {
                let databases = self.database_manager.get_databases();
                // Use dropdown index if dropdown is expanded, otherwise use selected index
                let index = if self.state.database_dropdown_expanded {
                    self.state.dropdown_selected_index
                } else {
                    self.selected_db_index
                };
                if let Some(db) = databases.get(index) {
                    self.state.start_database_delete_confirmation(db.name.clone());
                }
            }
            NavigationPanel::TableList => {
                let current_tables = self.get_current_tables();
                if let Some(table) = current_tables.get(self.selected_table_index) {
                    self.state.start_table_delete_confirmation(table.clone());
                }
            }
            _ => {}
        }
    }

    fn immediate_delete(&mut self) {
        match self.state.active_panel {
            NavigationPanel::DatabaseList => {
                let db_name = {
                    let databases = self.database_manager.get_databases();
                    // Use dropdown index if dropdown is expanded, otherwise use selected index
                    let index = if self.state.database_dropdown_expanded {
                        self.state.dropdown_selected_index
                    } else {
                        self.selected_db_index
                    };
                    databases.get(index).map(|db| db.name.clone())
                };
                if let Some(name) = db_name {
                    self.delete_database(&name);
                }
            }
            NavigationPanel::TableList => {
                let table_name = {
                    let current_tables = self.get_current_tables();
                    current_tables.get(self.selected_table_index).cloned()
                };
                if let Some(name) = table_name {
                    self.delete_table(&name);
                }
            }
            _ => {}
        }
    }

    fn confirm_delete(&mut self) {
        let (item_type, item_name) = match &self.state.delete_confirmation {
            crate::app::state::DeleteConfirmationState::Database(name) => ("database", name.clone()),
            crate::app::state::DeleteConfirmationState::Table(name) => ("table", name.clone()),
            _ => return,
        };
        
        match item_type {
            "database" => self.delete_database(&item_name),
            "table" => self.delete_table(&item_name),
            _ => {}
        }
    }

    fn delete_database(&mut self, db_name: &str) {
        if let Err(e) = self.database_manager.remove_database(db_name) {
            self.state.show_error(format!("Failed to delete database: {e}"));
        } else {
            self.sync_selected_db_index();
            self.selected_table_index = 0;
            self.state.show_success(format!("Deleted database '{db_name}'"));
        }
    }

    fn delete_table(&mut self, table_name: &str) {
        if let Err(e) = self.database_manager.remove_table(table_name) {
            self.state.show_error(format!("Failed to delete table: {e}"));
        } else {
            // Clear table data if we deleted the currently viewed table
            if let Some(current_table) = &self.state.selected_table {
                if current_table == table_name {
                    self.state.table_data = None;
                    self.state.selected_table = None;
                }
            }
            self.sync_selected_table_index();
            self.state.show_success(format!("Deleted table '{table_name}'"));
        }
    }

    fn save_current_database_to_file(&mut self) {
        if let Some(current_db) = self.database_manager.get_current_database() {
            let db_name = current_db.to_string();
            let filename = self.state.save_filename.clone();
            
            // Add .db extension if not present
            let file_path = if filename.ends_with(".db") || filename.ends_with(".duckdb") {
                filename
            } else {
                format!("{filename}.db")
            };
            
            let mut workflows = DatabaseWorkflows::new(
                &mut self.database_manager,
                &mut self.action_logger,
                &mut self.state,
            );
            
            match workflows.save_database_to_file(db_name, std::path::PathBuf::from(file_path)) {
                Ok(_) => {
                    // Success message handled by workflow
                }
                Err(e) => {
                    self.state.show_error(format!("Failed to save database: {e}"));
                }
            }
        } else {
            self.state.show_error("No database selected to save".to_string());
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

        // Main content area - New 2-panel layout
        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(30), // Left sidebar
                Constraint::Percentage(70), // Table viewer
            ])
            .split(chunks[1]);

        // Left sidebar (combined database + table list)
        self.render_left_sidebar(f, main_chunks[0]);

        // Table viewer (renamed from main content)
        self.render_table_viewer(f, main_chunks[1]);

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

        // Render database dropdown overlay if expanded
        if self.state.database_dropdown_expanded {
            self.render_database_dropdown_overlay(f, f.area());
        }

        // Render database name input popup
        if self.state.is_entering_database_name {
            self.render_database_name_input(f, f.area());
        }

        // Render save filename input popup
        if self.state.is_entering_save_filename {
            self.render_save_filename_input(f, f.area());
        }

        // Render delete confirmation popup
        if self.state.is_delete_confirmation_active() {
            self.render_delete_confirmation(f, f.area());
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

        // Render database dropdown in top area
        self.render_database_dropdown(f, sidebar_chunks[0]);
        
        // Render table list in bottom area
        self.render_table_list_compact(f, sidebar_chunks[1]);
    }

    fn render_database_dropdown(&self, f: &mut Frame, area: Rect) {
        let current_db = self.database_manager.get_current_database().unwrap_or("none");
        
        // Always render collapsed state here - expanded state is handled as overlay
        let content = format!("DB: [{}]", current_db);
        
        let border_style = self.get_panel_border_style(NavigationPanel::DatabaseList);

        let dropdown = Paragraph::new(content)
            .block(
                Block::default()
                    .title("Database")
                    .borders(Borders::ALL)
                    .border_style(border_style),
            )
            .style(Style::default().fg(Color::White));

        f.render_widget(dropdown, area);
    }

    fn render_table_list_compact(&self, f: &mut Frame, area: Rect) {
        let current_tables = self.get_current_tables();
        
        let items: Vec<ListItem> = current_tables
            .iter()
            .enumerate()
            .map(|(i, table)| {
                let is_selected = i == self.selected_table_index;
                let is_current = self.state.selected_table.as_ref().map_or(false, |current| current == table);
                
                let style = if is_selected {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else if is_current {
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                
                let selection_indicator = if is_current { "● " } else { "  " };
                let display_name = format!("{}📋 {}", selection_indicator, table);
                ListItem::new(display_name).style(style)
            })
            .collect();

        let border_style = self.get_panel_border_style(NavigationPanel::TableList);

        let list = List::new(items)
            .block(
                Block::default()
                    .title("Tables")
                    .borders(Borders::ALL)
                    .border_style(border_style),
            )
            .style(Style::default().fg(Color::White));

        f.render_widget(list, area);
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

    fn render_table_viewer(&self, f: &mut Frame, area: Rect) {
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
                self.render_table_widget(f, area, data, &format!("Table: {} ({} rows)", table, data.row_count));
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

    fn render_status_bar(&self, f: &mut Frame, area: Rect) {
        // Split status bar into left and right sections
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
                self.state.selected_col + 1,
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

    fn render_database_name_input(&self, f: &mut Frame, area: Rect) {
        // Create centered popup
        let popup_width = 50;
        let popup_height = 5;
        let x = (area.width.saturating_sub(popup_width)) / 2;
        let y = (area.height.saturating_sub(popup_height)) / 2;

        let popup_area = Rect {
            x,
            y,
            width: popup_width,
            height: popup_height,
        };

        let content = format!(
            "Create New Database\n\nName: {}\n\nPress Enter to create, Esc to cancel",
            if self.state.new_database_name.is_empty() {
                "_"
            } else {
                &self.state.new_database_name
            }
        );

        let popup = Paragraph::new(content)
            .block(
                Block::default()
                    .title("Database Name")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            )
            .style(Style::default().fg(Color::White).bg(Color::Black).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center);

        f.render_widget(popup, popup_area);
    }

    fn render_save_filename_input(&self, f: &mut Frame, area: Rect) {
        // Create centered popup
        let popup_width = 60;
        let popup_height = 6;
        let x = (area.width.saturating_sub(popup_width)) / 2;
        let y = (area.height.saturating_sub(popup_height)) / 2;

        let popup_area = Rect {
            x,
            y,
            width: popup_width,
            height: popup_height,
        };

        let current_db = self.database_manager.get_current_database().unwrap_or("none");
        let display_filename = if self.state.save_filename.is_empty() {
            "_"
        } else {
            &self.state.save_filename
        };

        let content = format!(
            "Save Database '{}' to File\n\nFilename: {}\n\n(.db extension will be added if not present)\nPress Enter to save, Esc to cancel",
            current_db,
            display_filename
        );

        let popup = Paragraph::new(content)
            .block(
                Block::default()
                    .title("Save Database")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            )
            .style(Style::default().fg(Color::White).bg(Color::Black).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center);

        f.render_widget(popup, popup_area);
    }

    fn render_delete_confirmation(&self, f: &mut Frame, area: Rect) {
        // Create centered popup
        let popup_width = 60;
        let popup_height = 6;
        let x = (area.width.saturating_sub(popup_width)) / 2;
        let y = (area.height.saturating_sub(popup_height)) / 2;

        let popup_area = Rect {
            x,
            y,
            width: popup_width,
            height: popup_height,
        };

        let (item_type, item_name) = match &self.state.delete_confirmation {
            crate::app::state::DeleteConfirmationState::Database(name) => ("database", name.as_str()),
            crate::app::state::DeleteConfirmationState::Table(name) => ("table", name.as_str()),
            _ => ("item", "unknown"),
        };

        let content = format!(
            "⚠️  Delete Confirmation\n\nDelete {} '{}'?\nThis action cannot be undone!\n\nPress 'd' to confirm, Esc to cancel",
            item_type, item_name
        );

        let popup = Paragraph::new(content)
            .block(
                Block::default()
                    .title("Confirm Delete")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            )
            .style(Style::default().fg(Color::White).bg(Color::Black).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center);

        f.render_widget(popup, popup_area);
    }

    fn render_database_dropdown_overlay(&self, f: &mut Frame, area: Rect) {
        let databases = self.database_manager.get_databases();
        if databases.is_empty() {
            return;
        }

        // Calculate dropdown area - positioned over the database dropdown widget
        
        // Calculate the database dropdown position within the sidebar
        let sidebar_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(0),    // Main content
                Constraint::Length(3), // Status bar
            ])
            .split(area);
        
        let content_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(30), // Left sidebar
                Constraint::Percentage(70), // Table viewer
            ])
            .split(sidebar_chunks[1]);
        
        let left_sidebar_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Database dropdown area
                Constraint::Min(0),    // Table list area
            ])
            .split(content_chunks[0]);

        // Position dropdown list below the database dropdown widget
        let dropdown_area = Rect {
            x: left_sidebar_chunks[0].x,
            y: left_sidebar_chunks[0].y + left_sidebar_chunks[0].height,
            width: left_sidebar_chunks[0].width,
            height: (databases.len() as u16 + 2).min(10), // Limit height, +2 for borders
        };

        // Ensure dropdown doesn't go beyond screen bounds
        let dropdown_area = Rect {
            x: dropdown_area.x,
            y: dropdown_area.y,
            width: dropdown_area.width,
            height: dropdown_area.height.min(area.height.saturating_sub(dropdown_area.y)),
        };

        // Create dropdown items
        let items: Vec<ListItem> = databases
            .iter()
            .enumerate()
            .map(|(i, db)| {
                let is_selected = i == self.state.dropdown_selected_index;
                let is_current = self.database_manager.get_current_database()
                    .map_or(false, |current| current == db.name);
                
                let style = if is_selected {
                    Style::default()
                        .fg(Color::Yellow)
                        .bg(Color::Blue)
                        .add_modifier(Modifier::BOLD)
                } else if is_current {
                    Style::default()
                        .fg(Color::Green)
                        .bg(Color::Black)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                        .fg(Color::White)
                        .bg(Color::Black)
                };
                
                let selection_indicator = if is_current { "● " } else { "  " };
                let display_name = format!("{}🗄️  {}", selection_indicator, db.name);
                ListItem::new(display_name).style(style)
            })
            .collect();

        // Create dropdown list widget
        let dropdown_list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            )
            .style(Style::default().fg(Color::White).bg(Color::Black));

        // Render with opaque black background to cover underlying widgets
        f.render_widget(dropdown_list, dropdown_area);
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
        format!("{}...", truncated)
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
