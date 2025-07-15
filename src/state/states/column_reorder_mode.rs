use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
};

use crate::state::{UIState, StateTransition, StateContext};

/// ColumnReorderMode handles column reordering sub-state
pub struct ColumnReorderMode;

impl ColumnReorderMode {
    pub fn new() -> Self {
        Self
    }
}

impl UIState for ColumnReorderMode {
    fn render(&self, frame: &mut Frame, area: Rect, is_active: bool, _context: &StateContext) -> Result<()> {
        let border_style = if is_active {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };
        
        let placeholder = Paragraph::new("Column Reorder Mode - TODO: Implement")
            .block(
                Block::default()
                    .title("Reorder Columns")
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
        "ColumnReorderMode"
    }
}

impl Default for ColumnReorderMode {
    fn default() -> Self {
        Self::new()
    }
}