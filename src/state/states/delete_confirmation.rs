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
    is_for_view: bool,  // true if deleting a view, false if deleting a table
}

impl DeleteConfirmation {
    pub fn new_for_table(table_name: String) -> Self {
        Self {
            target_name: table_name,
            is_for_database: false,
            is_for_view: false,
        }
    }
    
    pub fn new_for_view(view_name: String) -> Self {
        Self {
            target_name: view_name,
            is_for_database: false,
            is_for_view: true,
        }
    }
    
    pub fn new_for_database(database_name: String) -> Self {
        Self {
            target_name: database_name,
            is_for_database: true,
            is_for_view: false,
        }
    }
}

impl UIState for DeleteConfirmation {
    fn render(&mut self, frame: &mut Frame, area: Rect, is_active: bool, _context: &mut StateContext) -> Result<()> {
        let border_style = if is_active {
            Style::default().fg(Color::Red)
        } else {
            Style::default().fg(Color::White)
        };
        
        let target_type = if self.is_for_database { 
            "database" 
        } else if self.is_for_view { 
            "view" 
        } else { 
            "table" 
        };
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
    
    fn handle_event(&mut self, event: KeyEvent, context: &mut StateContext) -> StateTransition {
        match event.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                // Perform actual deletion
                let result = if self.is_for_database {
                    // Database deletion logic would go here (not implemented yet)
                    context.set_status_message("Database deletion not implemented yet".to_string());
                    Ok(())
                } else if self.is_for_view {
                    // Delete view
                    context.database_manager.remove_view(&self.target_name)
                } else {
                    // Delete table
                    context.database_manager.remove_table(&self.target_name)
                };
                
                match result {
                    Ok(()) => {
                        let target_type = if self.is_for_view { "View" } else { "Table" };
                        context.set_status_message(format!("{} '{}' deleted successfully", target_type, self.target_name));
                    }
                    Err(e) => {
                        let target_type = if self.is_for_view { "view" } else { "table" };
                        context.set_status_message(format!("Failed to delete {}: {}", target_type, e));
                    }
                }
                
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