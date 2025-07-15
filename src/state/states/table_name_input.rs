use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
};

use crate::state::{UIState, StateTransition, StateContext};

/// TableNameInput handles table/database name input modal
pub struct TableNameInput {
    is_for_database: bool,
}

impl TableNameInput {
    pub fn new_for_table() -> Self {
        Self {
            is_for_database: false,
        }
    }
    
    pub fn new_for_database() -> Self {
        Self {
            is_for_database: true,
        }
    }
}

impl UIState for TableNameInput {
    fn render(&self, frame: &mut Frame, area: Rect, is_active: bool, _context: &StateContext) -> Result<()> {
        let border_style = if is_active {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };
        
        let title = if self.is_for_database {
            "Create Database"
        } else {
            "Create Table"
        };
        
        let placeholder = Paragraph::new("Name Input Modal - TODO: Implement")
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(border_style)
            )
            .style(Style::default().fg(Color::White));
        
        frame.render_widget(placeholder, area);
        Ok(())
    }
    
    fn handle_event(&mut self, event: KeyEvent, _context: &mut StateContext) -> StateTransition {
        match event.code {
            KeyCode::Esc => StateTransition::Pop,
            _ => StateTransition::Stay,
        }
    }
    
    fn debug_name(&self) -> &'static str {
        if self.is_for_database {
            "DatabaseNameInput"
        } else {
            "TableNameInput"
        }
    }
}