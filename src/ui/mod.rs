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
use file_browser::{render_file_browser_popup, FileBrowser};

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

        Self {
            state: ApplicationState::new(),
            database_manager,
            selected_db_index: 0,
            selected_table_index: 0,
            file_browser: None,
            show_file_browser: false,
            action_logger,
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
                            // Check if we're in table creation mode
                            if self.state.is_creating_table && self.state.table_creation_step == TableCreationStep::SelectingFile {
                                // Handle file import for table creation
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
                                    }
                                    Err(e) => {
                                        self.state.set_status(format!("Import failed: {}", e));
                                        self.state.complete_table_creation(false);
                                    }
                                }
                            } else {
                                // User selected a database file - use action system
                                let mut workflows = DatabaseWorkflows::new(
                                    &mut self.database_manager,
                                    &mut self.action_logger,
                                    &mut self.state,
                                );
                                let _ = workflows.select_file(selected_path);
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
                self.selected_db_index = 0;
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
                    self.selected_db_index = 0;
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
                }
            }
            _ => {}
        }
    }

    fn start_table_creation(&mut self) {
        if self.database_manager.get_current_database().is_some() {
            self.state.start_table_creation();
        } else {
            self.state.set_status("Please connect to a database first".to_string());
        }
    }

    fn show_help(&mut self) {
        self.state.set_status("Keys: Tab/Shift+Tab: Navigate panels | ↑↓: Select items | Enter: Confirm | i: Import data | o: Open file | n: New DB | d: Disconnect | h: Help | q/Esc: Quit".to_string());
    }

    fn open_file_browser(&mut self) {
        match FileBrowser::new() {
            Ok(browser) => {
                self.file_browser = Some(browser);
                self.show_file_browser = true;
                self.state
                    .set_status("File browser opened. Use Esc to close.".to_string());
            }
            Err(e) => {
                self.state
                    .set_status(format!("Error opening file browser: {e}"));
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

        // Render file browser popup if shown
        if self.show_file_browser {
            if let Some(ref browser) = self.file_browser {
                render_file_browser_popup(f, f.area(), browser);
            }
        }
    }

    fn render_database_list(&self, f: &mut Frame, area: Rect) {
        let databases = self.database_manager.get_databases();
        let items: Vec<ListItem> = databases
            .iter()
            .enumerate()
            .map(|(i, db)| {
                let style = if i == self.selected_db_index {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                let display_name = if db.is_memory {
                    format!("🧠 {}", db.name)
                } else {
                    format!("💾 {}", db.name)
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
                let style = if i == self.selected_table_index {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                let display_name = format!("📋 {table}");
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

        let (content, title) = if self.state.current_state == AppState::ImportWizard && self.state.is_creating_table {
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
                    (content, "Import Wizard - Table Name [3]")
                }
                TableCreationStep::SelectingFile => {
                    let content = format!(
                        "Create New Table: '{}'\n\nSelect a file to import data from:\n• CSV files (.csv)\n• JSON files (.json)\n• Parquet files (.parquet)\n\nPress Esc to cancel",
                        self.state.new_table_name
                    );
                    (content, "Import Wizard - File Selection [3]")
                }
                TableCreationStep::ImportingData => {
                    let content = format!(
                        "Create New Table: '{}'\n\nImporting data...\n\nPlease wait while the data is being imported.",
                        self.state.new_table_name
                    );
                    (content, "Import Wizard - Importing [3]")
                }
            }
        } else if let (Some(db), Some(table)) = (&self.state.selected_database, &self.state.selected_table) {
            let content = format!(
                "Database: {db}\nTable: {table}\n\nTable data will be displayed here..."
            );
            (content, "Main Content [3]")
        } else {
            let content = "Select a database and table to view data\n\nNavigation:\n• Use Tab/Shift+Tab to switch panels\n• Use ↑↓ to navigate lists\n• Press Enter to select items\n• Press i to import data\n• Press h for help\n• Press q or Esc to quit".to_string();
            (content, "Main Content [3]")
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

        let status = Paragraph::new(self.state.status_message.clone())
            .block(
                Block::default()
                    .title("Status")
                    .borders(Borders::ALL)
                    .border_style(border_style),
            )
            .style(Style::default().fg(Color::Cyan));

        f.render_widget(status, area);
    }
}
