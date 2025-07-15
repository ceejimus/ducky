use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
};

use crate::state::{UIState, StateTransition, StateContext};

/// TableInspector manages the schema and statistics display for tables
pub struct TableInspector {
    // TODO: Add inspector state (section selection, scrolling, etc.)
}

impl TableInspector {
    pub fn new() -> Self {
        Self {
            // TODO: Initialize state
        }
    }
}

impl UIState for TableInspector {
    fn render(&self, frame: &mut Frame, area: Rect, is_active: bool, context: &StateContext) -> Result<()> {
        let border_style = if is_active {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };
        
        let title = if let Some(table_name) = &context.global_state.selected_table {
            format!("Inspector: {}", table_name)
        } else {
            "Inspector".to_string()
        };
        
        let placeholder = Paragraph::new("Table inspector - TODO: Implement schema/stats display")
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
            KeyCode::Esc => {
                // Go back to data viewer (replace current main panel content)
                StateTransition::Replace(Box::new(super::TableDataViewer::new()))
            }
            KeyCode::Left | KeyCode::Char('h') => {
                // Focus the table panel
                StateTransition::FocusPanel(crate::state::PanelType::Table)
            }
            _ => StateTransition::Stay,
        }
    }
    
    fn debug_name(&self) -> &'static str {
        "TableInspector"
    }
}

impl Default for TableInspector {
    fn default() -> Self {
        Self::new()
    }
}