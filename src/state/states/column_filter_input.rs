use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
};

use crate::state::{UIState, StateTransition, StateContext};

/// ColumnFilterInput handles column filter input modal with real-time search
pub struct ColumnFilterInput {
    syntax_valid: bool,
}

impl ColumnFilterInput {
    pub fn new() -> Self {
        Self {
            syntax_valid: true,
        }
    }
    
    /// Validate filter syntax by testing a query
    fn validate_syntax(&mut self, context: &StateContext) {
        // If search text is empty, consider it valid
        if context.global_state.search_text.trim().is_empty() {
            self.syntax_valid = true;
            return;
        }
        
        // Try to validate by building a test query
        if let (Some(column_name), Some(table_name)) = 
            (&context.global_state.search_column, &context.global_state.selected_table) {
            
            let test_sql = format!(
                "SELECT COUNT(*) FROM {} WHERE {} {}",
                table_name, column_name, context.global_state.search_text.trim()
            );

            // Try to prepare the statement to validate syntax
            if let Some(current_db) = context.database_manager.get_current_database() {
                if let Some(connection) = context.database_manager.get_connection(current_db) {
                    match connection.prepare(&test_sql) {
                        Ok(_) => {
                            self.syntax_valid = true;
                        }
                        Err(_) => {
                            self.syntax_valid = false;
                        }
                    }
                } else {
                    self.syntax_valid = false;
                }
            } else {
                self.syntax_valid = false;
            }
        } else {
            self.syntax_valid = false;
        }
    }
    
}

impl UIState for ColumnFilterInput {
    fn render(&mut self, frame: &mut Frame, area: Rect, is_active: bool, context: &mut StateContext) -> Result<()> {
        // Validate syntax on every render
        self.validate_syntax(context);
        
        let border_style = if is_active {
            if self.syntax_valid {
                Style::default().fg(Color::Green)  // Green border for valid syntax
            } else {
                Style::default().fg(Color::Red)    // Red border for invalid syntax
            }
        } else {
            Style::default().fg(Color::White)
        };
        
        let title = if let Some(column_name) = &context.global_state.search_column {
            format!("Filter: {}", column_name)
        } else {
            "Filter Column".to_string()
        };
        
        let search_text = if context.global_state.is_searching {
            format!("{}|", context.global_state.search_text)
        } else {
            context.global_state.search_text.clone()
        };
        
        let status = if self.syntax_valid {
            "Valid SQL syntax"
        } else {
            "Invalid SQL syntax"
        };
        
        let content = format!("{}\n\nExamples: = 'value', LIKE '%pattern%', > 100\nStatus: {}\n\nPress Enter to apply, Esc to cancel", 
                             search_text, status);
        
        let input_widget = Paragraph::new(content)
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(border_style)
            )
            .style(Style::default().fg(Color::White));
        
        frame.render_widget(input_widget, area);
        Ok(())
    }
    
    fn handle_event(&mut self, event: KeyEvent, context: &mut StateContext) -> StateTransition {
        match event.code {
            KeyCode::Esc => {
                context.global_state.cancel_filter();
                StateTransition::Pop
            }
            KeyCode::Enter => {
                if self.syntax_valid && !context.global_state.search_text.trim().is_empty() {
                    context.global_state.apply_filter();
                    // Note: The parent TableDataViewer will need to refresh data when this modal is popped
                    StateTransition::Pop
                } else {
                    // Don't apply filter if syntax is invalid or empty
                    StateTransition::Stay
                }
            }
            KeyCode::Backspace => {
                if !context.global_state.search_text.is_empty() {
                    context.global_state.search_text.pop();
                    context.global_state.reset_debounce_timer();
                }
                StateTransition::Stay
            }
            KeyCode::Char(c) => {
                context.global_state.search_text.push(c);
                context.global_state.reset_debounce_timer();
                StateTransition::Stay
            }
            _ => StateTransition::Stay,
        }
    }
    
    fn debug_name(&self) -> &'static str {
        "ColumnFilterInput"
    }
}

impl Default for ColumnFilterInput {
    fn default() -> Self {
        Self::new()
    }
}