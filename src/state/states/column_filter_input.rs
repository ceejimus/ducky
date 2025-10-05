use crate::db::DatabaseManager;
use crate::state::{AppContext, StateTransition, UIState};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};
use std::time::Instant;

pub struct ColumnFilterInputState {
    column_name: String,
    search_text: String,
    search_syntax_valid: bool,
    debounce_timer: Option<Instant>,
}

impl ColumnFilterInputState {
    pub fn new(column_name: String) -> Self {
        Self {
            column_name,
            search_text: String::new(),
            search_syntax_valid: true,
            debounce_timer: None,
        }
    }

    // Exact copy from ui/mod.rs validate_search_syntax (lines 1568-1606)
    fn validate_syntax(&mut self, context: &AppContext, db_manager: &DatabaseManager) {
        // If search text is empty, consider it valid
        if self.search_text.trim().is_empty() {
            self.search_syntax_valid = true;
            return;
        }

        // Try to validate by building a test query (EXACT copy from legacy)
        if let Some(ref table) = context.selected_table {
            let test_sql = format!(
                "SELECT COUNT(*) FROM {} WHERE {} {}",
                table, self.column_name, self.search_text.trim()
            );

            // Try to prepare the statement to validate syntax
            if let Some(connection) = db_manager.get_current_connection() {
                match connection.prepare(&test_sql) {
                    Ok(_) => {
                        self.search_syntax_valid = true;
                    }
                    Err(_) => {
                        self.search_syntax_valid = false;
                    }
                }
            } else {
                // No connection available, assume valid for now
                self.search_syntax_valid = true;
            }
        }
    }

    fn apply_filter(&self, context: &mut AppContext) {
        if self.search_syntax_valid && !self.search_text.trim().is_empty() {
            // Store the filter in context
            context.column_filters.insert(
                self.column_name.clone(),
                self.search_text.trim().to_string(),
            );
        }
    }
}

impl UIState for ColumnFilterInputState {
    fn render(&self, frame: &mut Frame, area: Rect, _context: &AppContext, _db_manager: &DatabaseManager) {
        // Determine border color based on syntax validity (exact copy from legacy)
        let border_color = if self.search_syntax_valid {
            Color::Green // Green border for valid syntax
        } else {
            Color::Red // Red border for invalid syntax
        };

        // Create search input display with cursor (exact copy from legacy)
        let search_display = format!("Filter [{}]: {}_", self.column_name, self.search_text);

        // Calculate available width (subtract borders and padding)
        let available_width = area.width.saturating_sub(4) as usize; // 2 for borders + 2 for padding
        let display_text = if search_display.len() > available_width {
            // Truncate if too long
            format!("{}...", &search_display[..available_width.saturating_sub(3)])
        } else {
            search_display
        };

        let search_input = Paragraph::new(display_text)
            .block(
                Block::default()
                    .title("Search/Filter")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(border_color)),
            )
            .style(Style::default().fg(Color::White));

        frame.render_widget(search_input, area);
    }

    fn handle_event(&mut self, event: KeyEvent, context: &mut AppContext, db_manager: &mut DatabaseManager) -> StateTransition {
        match event.code {
            KeyCode::Esc => {
                // Cancel search - don't apply filter
                StateTransition::Pop
            }
            KeyCode::Enter => {
                // Apply filter if valid
                if self.search_syntax_valid {
                    self.apply_filter(context);
                }
                StateTransition::Pop
            }
            KeyCode::Backspace => {
                self.search_text.pop();
                self.validate_syntax(context, db_manager);
                self.debounce_timer = Some(Instant::now());
                StateTransition::Stay
            }
            KeyCode::Char(c) => {
                self.search_text.push(c);
                self.validate_syntax(context, db_manager);
                self.debounce_timer = Some(Instant::now());
                StateTransition::Stay
            }
            _ => StateTransition::Stay,
        }
    }

    fn name(&self) -> &'static str {
        "ColumnFilterInputState"
    }

    fn on_enter(&mut self, _context: &mut AppContext, _db_manager: &mut DatabaseManager) {
        // Clear any previous search text when entering
        self.search_text.clear();
        self.search_syntax_valid = true;
        self.debounce_timer = None;
    }

    fn on_exit(&mut self, _context: &mut AppContext, _db_manager: &mut DatabaseManager) {
        // Clean up when exiting
        self.search_text.clear();
        self.debounce_timer = None;
    }
}
