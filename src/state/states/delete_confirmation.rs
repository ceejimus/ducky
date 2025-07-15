use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
};

use crate::state::{UIState, StateTransition, StateContext};

/// DeleteConfirmation handles delete confirmation modal
pub struct DeleteConfirmation {
    target_name: String,
    is_for_database: bool,
}

impl DeleteConfirmation {
    pub fn new_for_table(table_name: String) -> Self {
        Self {
            target_name: table_name,
            is_for_database: false,
        }
    }
    
    pub fn new_for_database(database_name: String) -> Self {
        Self {
            target_name: database_name,
            is_for_database: true,
        }
    }
}

impl UIState for DeleteConfirmation {
    fn render(&self, frame: &mut Frame, area: Rect, is_active: bool, _context: &StateContext) -> Result<()> {
        let border_style = if is_active {
            Style::default().fg(Color::Red)
        } else {
            Style::default().fg(Color::White)
        };
        
        let target_type = if self.is_for_database { "database" } else { "table" };
        let message = format!("Delete {} '{}'? (y/N)", target_type, self.target_name);
        
        let confirmation = Paragraph::new(message)
            .block(
                Block::default()
                    .title("Confirm Delete")
                    .borders(Borders::ALL)
                    .border_style(border_style)
            )
            .style(Style::default().fg(Color::White));
        
        frame.render_widget(confirmation, area);
        Ok(())
    }
    
    fn handle_event(&mut self, event: KeyEvent, _context: &mut StateContext) -> StateTransition {
        match event.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                // TODO: Perform actual deletion
                StateTransition::Pop
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                StateTransition::Pop
            }
            _ => StateTransition::Stay,
        }
    }
    
    fn debug_name(&self) -> &'static str {
        if self.is_for_database {
            "DatabaseDeleteConfirmation"
        } else {
            "TableDeleteConfirmation"
        }
    }
}