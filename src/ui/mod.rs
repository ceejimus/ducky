use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
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
        let mut database_manager = DatabaseManager::new();

        // Initialize ActionLogger
        let mut action_logger = ActionLogger::new().unwrap_or_else(|e| {
            panic!("Warning: Failed to initialize action logger: {e}");
            // Create a dummy logger that doesn't write to file
            // ActionLogger::new().unwrap()
        });

        // Initialize with default databases
        if let Err(e) = database_manager.initialize_default_databases() {
            action_logger.log_error(&format!("Failed to initialize default databases: {e}"));
        } else {
            action_logger.log_info("Default databases initialized successfully");
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
                                                    self.state.show_error(format!("Import failed: {}", e));
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
            KeyCode::Tab => self.state.next_panel(),
            KeyCode::BackTab => self.state.prev_panel(),
            KeyCode::Up => self.handle_up(),
            KeyCode::Down => self.handle_down(),
            KeyCode::Left => self.handle_left(),
            KeyCode::Right => self.handle_right(),
            KeyCode::Enter => self.handle_enter(),
            KeyCode::Char('h') => self.show_help(),
            KeyCode::Char('i') => self.start_table_creation(),
            KeyCode::Char('o') => self.open_file_browser(),
            KeyCode::Char('n') => {
                let mut workflows = DatabaseWorkflows::new(
                    &mut self.database_manager,
                    &mut self.action_logger,
                    &mut self.state,
                );
                let _ = workflows.create_new_database();
                self.sync_selected_db_index();
                self.selected_table_index = 0;
            }
            KeyCode::Char('d') => {
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
            KeyCode::Char('1') => self.state.active_panel = NavigationPanel::DatabaseList,
            KeyCode::Char('2') => self.state.active_panel = NavigationPanel::TableList,
            KeyCode::Char('3') => self.state.active_panel = NavigationPanel::MainContent,
            _ => {}
        }
    }

    fn handle_up(&mut self) {
        match self.state.active_panel {
            NavigationPanel::DatabaseList => {
                if self.selected_db_index > 0 {
                    self.selected_db_index -= 1;
                }
            }
            NavigationPanel::TableList => {
                if self.selected_table_index > 0 {
                    self.selected_table_index -= 1;
                }
            }
            NavigationPanel::MainContent => {
                // Scroll table data up
                if self.state.table_data.is_some() {
                    self.state.scroll_table_up();
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
                if self.selected_db_index < databases.len().saturating_sub(1) {
                    self.selected_db_index += 1;
                }
            }
            NavigationPanel::TableList => {
                if self.selected_table_index < current_tables.len().saturating_sub(1) {
                    self.selected_table_index += 1;
                }
            }
            NavigationPanel::MainContent => {
                // Scroll table data down
                if let Some(ref data) = self.state.table_data {
                    // Calculate visible rows to pass to scroll method
                    let visible_rows = 10; // Approximate - could be calculated from area
                    self.state.scroll_table_down(data.rows.len(), visible_rows);
                }
            }
            _ => {}
        }
    }

    fn handle_left(&mut self) {
        match self.state.active_panel {
            NavigationPanel::MainContent => {
                // Scroll table data left
                if self.state.table_data.is_some() {
                    self.state.scroll_table_left();
                }
            }
            _ => {}
        }
    }

    fn handle_right(&mut self) {
        match self.state.active_panel {
            NavigationPanel::MainContent => {
                // Scroll table data right
                if let Some(ref data) = self.state.table_data {
                    // Calculate visible columns to pass to scroll method
                    let visible_cols = 5; // Approximate - could be calculated from area
                    self.state.scroll_table_right(data.columns.len(), visible_cols);
                }
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
                self.state.show_error(format!("Failed to refresh database: {}", e));
            }
        }
    }

    fn render_table_data(&self, data: &crate::db::query::QueryResult, area: Rect) -> String {
        // Calculate available space (subtract borders)
        let available_width = area.width.saturating_sub(2) as usize;
        let available_height = area.height.saturating_sub(2) as usize;
        
        let mut content = String::new();
        
        // Calculate column widths
        let col_width = if data.columns.is_empty() {
            12
        } else {
            (available_width.saturating_sub(data.columns.len() + 1)) / data.columns.len().max(1)
        };
        let col_width = col_width.max(8).min(20); // Min 8, max 20 characters per column
        
        // Determine visible columns based on scroll position
        let visible_cols = (available_width / (col_width + 1)).max(1);
        let start_col = self.state.scroll_x;
        let end_col = (start_col + visible_cols).min(data.columns.len());
        
        // Render header row (frozen)
        let header_line = data.columns[start_col..end_col]
            .iter()
            .map(|col| format!("{:width$}", truncate_text(col, col_width), width = col_width))
            .collect::<Vec<_>>()
            .join("|");
        content.push_str(&header_line);
        content.push('\n');
        
        // Add separator line
        let separator = data.columns[start_col..end_col]
            .iter()
            .map(|_| "-".repeat(col_width))
            .collect::<Vec<_>>()
            .join("+");
        content.push_str(&separator);
        content.push('\n');
        
        // Render data rows
        let visible_rows = available_height.saturating_sub(2); // Subtract header and separator
        let start_row = self.state.scroll_y;
        let end_row = (start_row + visible_rows).min(data.rows.len());
        
        for row in &data.rows[start_row..end_row] {
            let row_line = row[start_col..end_col.min(row.len())]
                .iter()
                .map(|cell| format!("{:width$}", truncate_text(cell, col_width), width = col_width))
                .collect::<Vec<_>>()
                .join("|");
            content.push_str(&row_line);
            content.push('\n');
        }
        
        // Add scroll info
        if data.rows.len() > visible_rows || data.columns.len() > visible_cols {
            content.push('\n');
            content.push_str(&format!(
                "Rows: {}-{} of {} | Cols: {}-{} of {} | Use arrows to scroll",
                start_row + 1,
                end_row,
                data.rows.len(),
                start_col + 1,
                end_col,
                data.columns.len()
            ));
        }
        
        content
    }

    fn fetch_table_data(&mut self) {
        if let (Some(_db), Some(table)) = (&self.state.selected_database, &self.state.selected_table) {
            if let Some(connection) = self.database_manager.get_current_connection() {
                let sql = format!("SELECT * FROM {} LIMIT 1000", table);
                match self.execute_query_direct(connection, &sql) {
                    Ok(data) => {
                        self.state.set_table_data(data);
                    }
                    Err(e) => {
                        self.state.show_error(format!("Failed to load table data: {}", e));
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
                .unwrap_or(&format!("column_{}", i))
                .to_string();
            columns.push(column_name);
        }
        
        // Collect all rows
        let mut result_rows = Vec::new();
        while let Some(row) = rows.next()? {
            let mut row_data = Vec::new();
            for i in 0..column_count {
                // Convert each column value to string with safer error handling
                let value = match row.get::<_, String>(i) {
                    Ok(v) => v,
                    Err(_) => match row.get::<_, i64>(i) {
                        Ok(v) => v.to_string(),
                        Err(_) => match row.get::<_, f64>(i) {
                            Ok(v) => v.to_string(),
                            Err(_) => match row.get::<_, bool>(i) {
                                Ok(v) => v.to_string(),
                                Err(_) => {
                                    // Try to get as raw value or default to NULL
                                    match row.get_ref(i) {
                                        Ok(value_ref) => format!("{:?}", value_ref),
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
                let db_name = {
                    let databases = self.database_manager.get_databases();
                    databases
                        .get(self.selected_db_index)
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
    }

    fn show_help(&mut self) {
        self.state.show_info("Keys: Tab/Shift+Tab=Navigate | ↑↓=Select | Enter=Confirm | i=Import | o=Open | n=New DB | d=Disconnect | q=Quit".to_string());
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

        // Main content area
        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25), // Database list
                Constraint::Percentage(25), // Table list
                Constraint::Percentage(50), // Main content
            ])
            .split(chunks[1]);

        // Database list
        self.render_database_list(f, main_chunks[0]);

        // Table list
        self.render_table_list(f, main_chunks[1]);

        // Main content
        self.render_main_content(f, main_chunks[2]);

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
    }

    fn render_database_list(&self, f: &mut Frame, area: Rect) {
        let databases = self.database_manager.get_databases();
        let current_db = self.database_manager.get_current_database();
        
        let items: Vec<ListItem> = databases
            .iter()
            .enumerate()
            .map(|(i, db)| {
                let is_selected = i == self.selected_db_index;
                let is_connected = current_db.map_or(false, |current| current == db.name);
                
                let style = if is_selected {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else if is_connected {
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                
                let connection_indicator = if is_connected { "● " } else { "  " };
                let display_name = if db.is_memory {
                    format!("{}🧠 {}", connection_indicator, db.name)
                } else {
                    format!("{}💾 {}", connection_indicator, db.name)
                };
                ListItem::new(display_name).style(style)
            })
            .collect();

        let is_active = matches!(self.state.active_panel, NavigationPanel::DatabaseList);
        let border_style = if is_active {
            Style::default().fg(Color::Green)
        } else {
            Style::default()
        };

        let list = List::new(items)
            .block(
                Block::default()
                    .title("Databases [1]")
                    .borders(Borders::ALL)
                    .border_style(border_style),
            )
            .style(Style::default().fg(Color::White));

        f.render_widget(list, area);
    }

    fn render_table_list(&self, f: &mut Frame, area: Rect) {
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

        let is_active = matches!(self.state.active_panel, NavigationPanel::TableList);
        let border_style = if is_active {
            Style::default().fg(Color::Green)
        } else {
            Style::default()
        };

        let list = List::new(items)
            .block(
                Block::default()
                    .title("Tables [2]")
                    .borders(Borders::ALL)
                    .border_style(border_style),
            )
            .style(Style::default().fg(Color::White));

        f.render_widget(list, area);
    }

    fn render_main_content(&self, f: &mut Frame, area: Rect) {
        let is_active = matches!(self.state.active_panel, NavigationPanel::MainContent);
        let border_style = if is_active {
            Style::default().fg(Color::Green)
        } else {
            Style::default()
        };

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
                    (content, "Import Wizard - Table Name [3]".to_string())
                }
                TableCreationStep::SelectingFile => {
                    let content = format!(
                        "Create New Table: '{}'\n\nSelect a file to import data from:\n• CSV files (.csv)\n• JSON files (.json)\n• Parquet files (.parquet)\n\nPress Esc to cancel",
                        self.state.new_table_name
                    );
                    (content, "Import Wizard - File Selection [3]".to_string())
                }
                TableCreationStep::ImportingData => {
                    let content = format!(
                        "Create New Table: '{}'\n\nImporting data...\n\nPlease wait while the data is being imported.",
                        self.state.new_table_name
                    );
                    (content, "Import Wizard - Importing [3]".to_string())
                }
            }
        } else if let (Some(db), Some(table)) = (&self.state.selected_database, &self.state.selected_table) {
            if let Some(ref data) = self.state.table_data {
                // Render table data
                let content = self.render_table_data(data, area);
                let title = format!("Table: {} ({} rows) [3]", table, data.row_count);
                (content, title)
            } else {
                let content = format!(
                    "Database: {db}\nTable: {table}\n\nLoading table data...\n\nPress Enter on table to load data or wait for auto-load"
                );
                (content, "Main Content [3]".to_string())
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
        let is_active = matches!(self.state.active_panel, NavigationPanel::StatusBar);
        let border_style = if is_active {
            Style::default().fg(Color::Green)
        } else {
            Style::default()
        };

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

            // Choose colors based on notification type
            let (border_color, text_color, icon) = match notification.notification_type {
                crate::app::state::NotificationType::Success => (Color::Green, Color::Green, "✅"),
                crate::app::state::NotificationType::Error => (Color::Red, Color::Red, "❌"),
                crate::app::state::NotificationType::Info => (Color::Blue, Color::Blue, "ℹ️"),
            };

            // Truncate notification text to fit
            let available_width = notification_width.saturating_sub(6) as usize; // Account for borders + icon
            let display_text = format!("{} {}", icon, truncate_text(&notification.message, available_width));

            let notification_widget = Paragraph::new(display_text)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(border_color))
                )
                .style(Style::default().fg(text_color).add_modifier(Modifier::BOLD))
                .alignment(Alignment::Center);

            // Clear the background area first
            let clear_block = Block::default().style(Style::default().bg(Color::Black));
            f.render_widget(clear_block, notification_area);
            
            // Render the notification
            f.render_widget(notification_widget, notification_area);
        }
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
