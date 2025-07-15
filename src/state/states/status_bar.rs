use anyhow::Result;
use crossterm::event::KeyEvent;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
};

use crate::state::{UIState, StateTransition, StateContext};

/// StatusBar displays status information and notifications
pub struct StatusBar;

impl StatusBar {
    pub fn new() -> Self {
        Self
    }
}

impl UIState for StatusBar {
    fn render(&self, frame: &mut Frame, area: Rect, is_active: bool, context: &StateContext) -> Result<()> {
        let border_style = if is_active {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Gray)
        };
        
        let status_text = &context.global_state.status_message;
        
        let status = Paragraph::new(status_text.as_str())
            .block(
                Block::default()
                    .title("Status")
                    .borders(Borders::ALL)
                    .border_style(border_style)
            )
            .style(Style::default().fg(Color::White));
        
        frame.render_widget(status, area);
        Ok(())
    }
    
    fn handle_event(&mut self, _event: KeyEvent, _context: &mut StateContext) -> StateTransition {
        // Status bar doesn't handle events directly
        StateTransition::Stay
    }
    
    fn debug_name(&self) -> &'static str {
        "StatusBar"
    }
}

impl Default for StatusBar {
    fn default() -> Self {
        Self::new()
    }
}