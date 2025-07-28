use crossterm::event::KeyCode;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem},
};

use crate::{
    db::{query::TableQuerySpec, DatabaseManager},
    state::{AppContext, StateKey, StateTransition, UIState},
};

pub struct TableListPanel {
    is_active: bool,
    table_list: Vec<String>,
    selected_table_index: usize,
}

impl TableListPanel {
    pub fn new() -> Self {
        Self {
            is_active: false,
            table_list: Vec::new(),
            selected_table_index: 0,
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

    fn get_current_tables(&self, db_manager: &DatabaseManager) -> Vec<String> {
        self.get_current_table_infos(db_manager)
            .into_iter()
            .map(|t| t.name.clone())
            .collect()
    }

    fn get_current_table_infos<'a>(
        &self,
        db_manager: &'a DatabaseManager,
    ) -> Vec<&'a crate::db::TableInfo> {
        if let Some(current_db) = db_manager.get_current_database() {
            if let Some(db_info) = db_manager.get_database_info(current_db) {
                return db_info.tables.iter().collect();
            }
        }
        Vec::new()
    }

    fn render_table_list(
        &self,
        frame: &mut Frame,
        area: Rect,
        context: &AppContext,
        db_manager: &DatabaseManager,
    ) {
        let current_table_infos = self.get_current_table_infos(db_manager);

        let items: Vec<ListItem> = current_table_infos
            .iter()
            .enumerate()
            .map(|(i, table_info)| {
                let is_selected = i == self.selected_table_index;
                let is_current = context.selected_table.as_ref() == Some(&table_info.name);

                // Choose icon based on table type
                let icon = if table_info.table_type == "VIEW" {
                    "[v]" // View indicator
                } else {
                    "[t]" // Table indicator
                };

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
                let display_name = format!("{}{} {}", selection_indicator, icon, table_info.name);
                ListItem::new(display_name).style(style)
            })
            .collect();

        let border_style = self.get_panel_border_style(context);

        let list = List::new(items)
            .block(
                Block::default()
                    .title("Tables")
                    .borders(Borders::ALL)
                    .border_style(border_style),
            )
            .style(Style::default().fg(Color::White));

        frame.render_widget(list, area);
    }

    fn select_table(
        &mut self,
        context: &mut AppContext,
        db_manager: &mut DatabaseManager,
    ) -> StateTransition {
        if let Some(table_name) = self.table_list.get(self.selected_table_index) {
            // Update context with selected table
            context.selected_table = Some(table_name.clone());

            // Clear table data when changing tables (matching legacy behavior)
            context.table_data = None;

            // Fetch table data using the new db interface
            self.fetch_table_data(context, db_manager, table_name);

            // Show success notification
            context.show_success(format!("Selected table: {table_name}"));

            // Transition to table data viewer
            StateTransition::To(StateKey::TableDataViewer)
        } else {
            StateTransition::Stay
        }
    }

    fn fetch_table_data(
        &self,
        context: &mut AppContext,
        db_manager: &DatabaseManager,
        table_name: &str,
    ) {
        // Create a basic TableQuerySpec for now (no sorting/filtering initially)
        let spec = TableQuerySpec::new(table_name.to_string());

        match db_manager.fetch_table_data(&spec) {
            Ok(data) => {
                context.table_data = Some(data);
            }
            Err(e) => {
                context.show_error(format!("Failed to load table data: {e}"));
            }
        }
    }
}

impl UIState for TableListPanel {
    fn render(
        &self,
        frame: &mut Frame,
        area: Rect,
        context: &AppContext,
        db_manager: &crate::db::DatabaseManager,
    ) {
        self.render_table_list(frame, area, context, db_manager);
    }

    fn handle_event(
        &mut self,
        event: crossterm::event::KeyEvent,
        context: &mut AppContext,
        db_manager: &mut crate::db::DatabaseManager,
    ) -> crate::state::StateTransition {
        match event.code {
            KeyCode::Tab | KeyCode::BackTab => StateTransition::To(StateKey::TableDataViewer),
            KeyCode::Char('k') => {
                if self.selected_table_index > 0 {
                    self.selected_table_index -= 1;
                }
                StateTransition::Stay
            }
            KeyCode::Char('j') => {
                if self.selected_table_index < self.table_list.len().saturating_sub(1) {
                    self.selected_table_index += 1;
                }
                StateTransition::Stay
            }
            KeyCode::Char('h') => StateTransition::To(StateKey::DatabaseSelect),
            KeyCode::Char('l') => StateTransition::To(StateKey::DatabaseSelect),
            KeyCode::Enter => {
                // Select table and transition to table data viewer
                self.select_table(context, db_manager)
            }
            _ => StateTransition::Stay, // Ignore unhandled keys
        }
    }

    fn name(&self) -> &'static str {
        "TableListPanel"
    }

    fn on_enter(&mut self, context: &mut AppContext, db_manager: &mut crate::db::DatabaseManager) {
        self.is_active = true;
        self.table_list = self.get_current_tables(db_manager);
    }

    fn on_exit(&mut self, context: &mut AppContext, db_manager: &mut crate::db::DatabaseManager) {
        self.is_active = false;
    }

    fn sync(&mut self, context: &AppContext, db_manager: &DatabaseManager) {
        // Update table list to current tables
        self.table_list = self.get_current_tables(db_manager);

        // Sync selected_table_index to match selected table (equivalent to sync_selected_table_index)
        if let Some(current_table) = &context.selected_table {
            if let Some(index) = self
                .table_list
                .iter()
                .position(|table| table == current_table)
            {
                self.selected_table_index = index;
            }
        }
    }
}
