use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

use crate::app::state::Notification;
use crate::state::{StateContext, StateTransition, UIState};

/// TableSelectPanel manages the table list and selection for the current database
pub struct TableSelectPanel {
    selected_index: usize,
}

impl TableSelectPanel {
    pub fn new() -> Self {
        Self { selected_index: 0 }
    }

    fn get_current_tables<'a>(&self, context: &'a StateContext) -> Vec<&'a crate::db::TableInfo> {
        if let Some(current_db) = &context.global_state.selected_database {
            if let Some(db_info) = context
                .database_manager
                .get_databases()
                .iter()
                .find(|db| &db.name == current_db)
            {
                return db_info.tables.iter().collect();
            }
        }
        Vec::new()
    }

    /// Update the selected table in the context
    fn update_selected_table(&self, context: &mut StateContext) {
        // Clone the table info to avoid borrowing issues
        let table_info = {
            let tables = self.get_current_tables(context);
            tables
                .get(self.selected_index)
                .map(|table| (table.name.clone(), table.column_count))
        };

        if let Some((table_name, column_count)) = table_info {
            context.set_selected_table(Some(table_name.clone()));
            tracing::info!("Selected table: {}", table_name);
            // hey claude! no UI state should directly control the context's status message
            // the context or the manager should handle setting this message from the current state
            // check all instances of this and other related separation of concern issues related
            // to the context
            context.set_status_message(format!("Table: {table_name} ({column_count} columns)"));
        }
    }
}

impl UIState for TableSelectPanel {
    fn render(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        is_active: bool,
        context: &mut StateContext,
    ) -> Result<()> {
        let tables = self.get_current_tables(context);

        let items: Vec<ListItem> = tables
            .iter()
            .enumerate()
            .map(|(i, table)| {
                let style = if i == self.selected_index {
                    if is_active {
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD)
                    }
                } else {
                    Style::default().fg(Color::White)
                };

                let display_text = match table.table_type.as_str() {
                    "VIEW" => format!("[v] {} ({} cols)", table.name, table.column_count),
                    _ => format!("[t] {} ({} cols)", table.name, table.column_count),
                };

                ListItem::new(display_text).style(style)
            })
            .collect();

        let border_style = if is_active {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };

        let title = if let Some(db_name) = &context.global_state.selected_database {
            format!("Tables ({db_name})")
        } else {
            "Tables".to_string()
        };

        let list = List::new(items)
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(border_style),
            )
            .style(Style::default().fg(Color::White));

        frame.render_widget(list, area);
        Ok(())
    }

    fn handle_event(&mut self, event: KeyEvent, context: &mut StateContext) -> StateTransition {
        match event.code {
            KeyCode::Up | KeyCode::Char('k') => {
                // hey claude! let's make this wrap around
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                    self.update_selected_table(context);
                }
                StateTransition::Stay
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let tables = self.get_current_tables(context);
                // hey claude! is the saturating_sub really necessary? it might be I just want to
                // know the reason
                if self.selected_index < tables.len().saturating_sub(1) {
                    self.selected_index += 1;
                    self.update_selected_table(context);
                }
                StateTransition::Stay
            }
            KeyCode::Enter => {
                self.update_selected_table(context);
                // Focus the main panel
                StateTransition::FocusPanel(crate::state::PanelType::Main)
            }
            KeyCode::Left | KeyCode::Char('h') | KeyCode::Right | KeyCode::Char('l') => {
                // The LeftSidebarPanel handles horizontal navigation between panels
                StateTransition::Stay
            }
            KeyCode::Char('i') => {
                // Import data - create new table
                StateTransition::Push(Box::new(super::TableNameInput::new_for_table()))
            }
            KeyCode::Char('v') => {
                // Create view from current table
                if self
                    .get_current_tables(context)
                    .get(self.selected_index)
                    .is_some()
                {
                    StateTransition::Push(Box::new(super::ViewNameInput::new()))
                } else {
                    StateTransition::Stay
                }
            }
            KeyCode::Delete | KeyCode::Char('d') => {
                let tables = self.get_current_tables(context);
                if let Some(table) = tables.get(self.selected_index) {
                    // Push delete confirmation modal - use appropriate type based on whether it's a view or table
                    let confirmation = if table.is_view() {
                        super::DeleteConfirmation::new_for_view(table.name.clone())
                    } else {
                        super::DeleteConfirmation::new_for_table(table.name.clone())
                    };
                    StateTransition::Push(Box::new(confirmation))
                } else {
                    StateTransition::Stay
                }
            }
            KeyCode::Char('r') => {
                // Refresh table list
                if let Some(db_name) = &context.global_state.selected_database {
                    // TODO: Implement refresh logic in database manager
                    tracing::info!("Refreshing tables for database: {}", db_name);
                    context
                        .add_notification(Notification::info("Table list refreshed".to_string()));
                }
                StateTransition::Stay
            }
            _ => StateTransition::Stay,
        }
    }

    fn on_enter(&mut self, context: &mut StateContext) -> Result<()> {
        tracing::debug!("Entered TableSelectPanel");
        self.update_selected_table(context);
        Ok(())
    }

    fn debug_name(&self) -> &'static str {
        "TableSelectPanel"
    }
}

impl Default for TableSelectPanel {
    fn default() -> Self {
        Self::new()
    }
}

