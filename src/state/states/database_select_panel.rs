use crate::db::DatabaseManager;
use crate::state::{AppContext, StateKey, StateTransition, UIState};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

pub struct DatabaseSelectPanel {
    pub selected_index: usize,
    pub dropdown_expanded: bool,
    pub dropdown_selected_index: usize,
    pub is_active: bool,
}

impl DatabaseSelectPanel {
    pub fn new() -> Self {
        Self {
            selected_index: 0,
            dropdown_expanded: false,
            dropdown_selected_index: 0,
            is_active: false,
        }
    }

    fn get_panel_border_style(&self, context: &AppContext) -> Style {
        if self.is_active {
            if context.is_panel_flashing() {
                // Flash effect: bright cyan (matching App's get_panel_border_style logic)
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                // Normal active: green (matching App's get_panel_border_style logic)
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            }
        } else {
            // Inactive panel: default (matching App's get_panel_border_style logic)
            Style::default()
        }
    }

    fn render_database_dropdown(
        &self,
        frame: &mut Frame,
        area: Rect,
        db_manager: &DatabaseManager,
        context: &AppContext,
    ) {
        let current_db = db_manager.get_current_database().unwrap_or("none");

        // Always render collapsed state here - expanded state is handled as overlay
        let content = format!("DB: [{current_db}]");

        let border_style = self.get_panel_border_style(context);
        let dropdown = Paragraph::new(content)
            .block(
                Block::default()
                    .title("Database")
                    .borders(Borders::ALL)
                    .border_style(border_style),
            )
            .style(Style::default().fg(Color::White));
        frame.render_widget(dropdown, area);
    }

    fn render_database_dropdown_overlay(
        &self,
        frame: &mut Frame,
        area: Rect,
        db_manager: &DatabaseManager,
    ) {
        let databases = db_manager.get_databases();
        if databases.is_empty() || !self.dropdown_expanded {
            return;
        }

        // Calculate dropdown area positioned below the database dropdown widget (matching legacy)
        let dropdown_height = (databases.len() as u16 + 2).min(10); // Limit height, +2 for borders
        let dropdown_area = Rect {
            x: area.x,
            y: area.y + 3, // Position below the dropdown widget
            width: area.width,
            height: dropdown_height,
        };

        // Ensure dropdown doesn't go beyond screen bounds (matching legacy)
        let dropdown_area = Rect {
            x: dropdown_area.x,
            y: dropdown_area.y,
            width: dropdown_area.width,
            height: dropdown_area
                .height
                .min(frame.area().height.saturating_sub(dropdown_area.y)),
        };

        // Create list items for databases (matching legacy styling and indicators)
        let items: Vec<ListItem> = databases
            .iter()
            .enumerate()
            .map(|(i, db)| {
                let is_selected = i == self.dropdown_selected_index;
                let is_current = db_manager
                    .get_current_database()
                    .is_some_and(|current| current == db.name);

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
                    Style::default().fg(Color::White).bg(Color::Black)
                };

                let selection_indicator = if is_current { "● " } else { "  " };
                let display_name = format!("{}🗄️  {}", selection_indicator, db.name);
                ListItem::new(display_name).style(style)
            })
            .collect();

        // Create dropdown list widget (matching legacy styling)
        let dropdown_list = List::new(items).block(
            Block::default().borders(Borders::ALL).border_style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        );

        frame.render_widget(dropdown_list, dropdown_area);
    }

    fn select_database(
        &mut self,
        context: &mut AppContext,
        db_manager: &mut DatabaseManager,
    ) -> StateTransition {
        let databases = db_manager.get_databases();
        if let Some(db) = databases.get(self.dropdown_selected_index) {
            let db_name = db.name.clone();
            if let Err(e) = db_manager.set_current_database(&db_name) {
                context.show_error(format!("Failed to connect to database: {e}"));
            } else {
                context.current_database = Some(db_name.clone());
                context.show_success(format!("Connected to database: {db_name}"));
                // Update selected index for consistency
                self.selected_index = self.dropdown_selected_index;
            }
        }
        self.dropdown_expanded = false;
        StateTransition::Stay
    }

    fn expand_dropdown(&mut self, num_databases: usize) {
        self.dropdown_expanded = true;
        // Ensure dropdown_selected_index is within bounds
        if self.dropdown_selected_index >= num_databases {
            self.dropdown_selected_index = 0;
        }
    }

    fn collapse_dropdown(&mut self) {
        self.dropdown_expanded = false;
    }

    fn dropdown_move_up(&mut self) {
        if self.dropdown_selected_index > 0 {
            self.dropdown_selected_index -= 1;
        }
    }

    fn dropdown_move_down(&mut self, num_databases: usize) {
        if self.dropdown_selected_index < num_databases.saturating_sub(1) {
            self.dropdown_selected_index += 1;
        }
    }

    fn set_dropdown_to_current_database(&mut self, current_db_index: usize) {
        self.dropdown_selected_index = current_db_index;
    }

    fn start_database_delete_confirmation(&self, db_manager: &DatabaseManager) -> StateTransition {
        let databases = db_manager.get_databases();
        if let Some(db) = databases.get(self.dropdown_selected_index) {
            let delete_target = crate::state::states::DeleteTarget::Database(db.name.clone());
            StateTransition::Push(crate::state::ModalKey::DeleteConfirmation(delete_target))
        } else {
            StateTransition::Stay
        }
    }

    fn immediate_database_delete(&self, context: &mut AppContext, db_manager: &mut DatabaseManager) -> StateTransition {
        let databases = db_manager.get_databases();
        if let Some(db) = databases.get(self.dropdown_selected_index) {
            let db_name = db.name.clone();
            match db_manager.remove_database(&db_name) {
                Ok(()) => {
                    context.show_success(format!("Deleted database '{}'", db_name));
                }
                Err(e) => {
                    context.show_error(format!("Failed to delete database: {}", e));
                }
            }
        }
        StateTransition::Stay
    }
}

impl UIState for DatabaseSelectPanel {
    fn render(
        &self,
        frame: &mut Frame,
        area: Rect,
        context: &AppContext,
        db_manager: &DatabaseManager,
    ) {
        // Render the main database dropdown
        self.render_database_dropdown(frame, area, db_manager, context);

        // Render overlay if expanded
        if self.dropdown_expanded {
            self.render_database_dropdown_overlay(frame, area, db_manager);
        }
    }

    fn handle_event(
        &mut self,
        event: KeyEvent,
        context: &mut AppContext,
        db_manager: &mut DatabaseManager,
    ) -> StateTransition {
        match event.code {
            KeyCode::Esc => {
                if self.dropdown_expanded {
                    // Close dropdown without making changes when Escape is pressed
                    self.collapse_dropdown();
                    self.set_dropdown_to_current_database(self.selected_index);
                }
                StateTransition::Stay
            }
            // Use vim-style navigation: j/k for up/down within database list
            KeyCode::Char('k') => {
                let databases = db_manager.get_databases();
                if !self.dropdown_expanded {
                    // Expand dropdown when first pressing j/k
                    self.expand_dropdown(databases.len());
                    self.set_dropdown_to_current_database(self.selected_index);
                } else {
                    // Navigate within dropdown (k = up)
                    self.dropdown_move_up();
                }
                StateTransition::Stay
            }
            KeyCode::Char('j') => {
                let databases = db_manager.get_databases();
                if !self.dropdown_expanded {
                    // Expand dropdown when first pressing j/k
                    self.expand_dropdown(databases.len());
                    self.set_dropdown_to_current_database(self.selected_index);
                } else {
                    // Navigate within dropdown (j = down)
                    self.dropdown_move_down(databases.len());
                }
                StateTransition::Stay
            }
            // h/l for panel navigation
            KeyCode::Char('h') => {
                log::debug!("DatabaseSelectPanel: handling 'h' key -> TableSelect");
                // h goes left - from database list, wrap to table list
                StateTransition::To(StateKey::TableSelect)
            }
            KeyCode::Char('l') => {
                log::debug!("DatabaseSelectPanel: handling 'l' key -> TableSelect");
                // l goes right - from database list, go to table list
                StateTransition::To(StateKey::TableSelect)
            }
            KeyCode::Enter => {
                if self.dropdown_expanded {
                    // Select database from dropdown
                    self.select_database(context, db_manager)
                } else {
                    // Expand dropdown if not already expanded
                    let databases = db_manager.get_databases();
                    self.expand_dropdown(databases.len());
                    self.set_dropdown_to_current_database(self.selected_index);
                    StateTransition::Stay
                }
            }
            KeyCode::Tab | KeyCode::BackTab => {
                if self.dropdown_expanded {
                    // Close dropdown without making changes when Tab is pressed
                    self.collapse_dropdown();
                    self.set_dropdown_to_current_database(self.selected_index);
                    StateTransition::Stay
                } else {
                    // Tab to next panel - from DatabaseList go to MainContent (same as next_panel logic)
                    StateTransition::To(StateKey::TableDataViewer)
                }
            }
            KeyCode::Char('d') => {
                if self.dropdown_expanded {
                    // Start delete confirmation for selected database in dropdown
                    self.start_database_delete_confirmation(db_manager)
                } else {
                    StateTransition::Stay
                }
            }
            KeyCode::Char('D') => {
                if self.dropdown_expanded {
                    // Immediate delete without confirmation for selected database in dropdown
                    self.immediate_database_delete(context, db_manager)
                } else {
                    StateTransition::Stay
                }
            }
            KeyCode::Char('n') => {
                // Start database name input (matching legacy behavior)
                StateTransition::Push(crate::state::ModalKey::DatabaseNameInput)
            }
            KeyCode::Char('q') => StateTransition::Exit,
            _ => StateTransition::Stay, // Ignore unhandled keys
        }
    }

    fn name(&self) -> &'static str {
        "DatabaseSelectPanel"
    }

    fn on_enter(&mut self, _context: &mut AppContext, _db_manager: &mut DatabaseManager) {
        self.is_active = true;
    }

    fn on_exit(&mut self, _context: &mut AppContext, _db_manager: &mut DatabaseManager) {
        self.is_active = false;
        // Close dropdown if it's open when exiting the panel
        if self.dropdown_expanded {
            self.collapse_dropdown();
            self.set_dropdown_to_current_database(self.selected_index);
        }
    }

    fn sync(&mut self, context: &AppContext, db_manager: &DatabaseManager) {
        // Sync selected_index to match current database (equivalent to sync_selected_db_index)
        if let Some(current_db) = db_manager.get_current_database() {
            let databases = db_manager.get_databases();
            if let Some(index) = databases.iter().position(|db| db.name == current_db) {
                self.selected_index = index;
            }
        }
    }
}

