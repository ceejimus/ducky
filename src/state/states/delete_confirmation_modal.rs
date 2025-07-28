use crossterm::event::KeyCode;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

use crate::{
    db::DatabaseManager,
    state::{AppContext, StateTransition, UIState},
};

#[derive(Debug, Clone, PartialEq)]
pub enum DeleteTarget {
    Database(String),
    Table(String),
}

pub struct DeleteConfirmationModal {
    target: DeleteTarget,
}

impl DeleteConfirmationModal {
    pub fn new(target: DeleteTarget) -> Self {
        Self { target }
    }

    fn get_confirmation_text(&self) -> (String, String) {
        match &self.target {
            DeleteTarget::Database(name) => (
                format!("Delete Database: {}", name),
                format!("Are you sure you want to delete the database '{}'?\n\nThis action cannot be undone.", name),
            ),
            DeleteTarget::Table(name) => (
                format!("Delete Table: {}", name),
                format!("Are you sure you want to delete the table '{}'?\n\nThis action cannot be undone.", name),
            ),
        }
    }

    fn execute_deletion(&self, context: &mut AppContext, db_manager: &mut DatabaseManager) -> Result<String, String> {
        match &self.target {
            DeleteTarget::Database(db_name) => {
                match db_manager.remove_database(db_name) {
                    Ok(()) => Ok(format!("Deleted database '{}'", db_name)),
                    Err(e) => Err(format!("Failed to delete database: {}", e)),
                }
            }
            DeleteTarget::Table(table_name) => {
                match db_manager.remove_table(table_name) {
                    Ok(()) => {
                        // Clear table data if we deleted the currently selected table
                        if context.selected_table.as_ref() == Some(table_name) {
                            context.selected_table = None;
                            context.table_data = None;
                        }
                        Ok(format!("Deleted table/view '{}'", table_name))
                    }
                    Err(e) => Err(format!("Failed to delete table/view: {}", e)),
                }
            }
        }
    }
}

impl UIState for DeleteConfirmationModal {
    fn render(
        &self,
        frame: &mut Frame,
        area: Rect,
        _context: &AppContext,
        _db_manager: &DatabaseManager,
    ) {
        // Create a centered popup area
        let popup_area = centered_rect(60, 40, area);
        
        // Clear the background
        frame.render_widget(Clear, popup_area);

        let (title, message) = self.get_confirmation_text();

        // Create the confirmation dialog
        let paragraph = Paragraph::new(format!("{}\n\nPress 'd' to confirm or 'q' to cancel", message))
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Red)),
            )
            .style(Style::default().fg(Color::White))
            .wrap(Wrap { trim: true })
            .alignment(Alignment::Center);

        frame.render_widget(paragraph, popup_area);
    }

    fn handle_event(
        &mut self,
        event: crossterm::event::KeyEvent,
        context: &mut AppContext,
        db_manager: &mut DatabaseManager,
    ) -> StateTransition {
        match event.code {
            KeyCode::Char('d') => {
                // Execute the deletion
                match self.execute_deletion(context, db_manager) {
                    Ok(success_msg) => {
                        context.show_success(success_msg);
                    }
                    Err(error_msg) => {
                        context.show_error(error_msg);
                    }
                }
                // Pop back to previous state
                StateTransition::Pop
            }
            KeyCode::Char('q') | KeyCode::Esc => {
                // Cancel deletion
                StateTransition::Pop
            }
            _ => StateTransition::Stay,
        }
    }

    fn name(&self) -> &'static str {
        "DeleteConfirmationModal"
    }
}

// Helper function to create a centered rectangle
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(r);

    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(popup_layout[1])[1]
}