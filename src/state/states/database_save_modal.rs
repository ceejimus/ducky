use crate::state::{UIState, StateTransition, AppContext};
use crate::db::DatabaseManager;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

pub struct DatabaseSaveModal {
    filename: String,
}

impl DatabaseSaveModal {
    pub fn new() -> Self {
        Self {
            filename: String::new(),
        }
    }
    
    fn save_database_to_file(&self, context: &mut AppContext, db_manager: &mut DatabaseManager) {
        if let Some(current_db) = db_manager.get_current_database() {
            let db_name = current_db.to_string();
            let filename = self.filename.clone();
            
            // Add .db extension if not present (matching legacy behavior)
            let file_path = if filename.ends_with(".db") || filename.ends_with(".duckdb") {
                filename
            } else {
                format!("{}.db", filename)
            };
            
            // Use DatabaseManager's save functionality directly
            match db_manager.save_database_to_file(&db_name, &file_path) {
                Ok(()) => {
                    context.show_success(format!("Saved database '{}' to '{}'", db_name, file_path));
                }
                Err(e) => {
                    context.show_error(format!("Failed to save database: {}", e));
                }
            }
        } else {
            context.show_error("No database selected to save".to_string());
        }
    }
}

impl UIState for DatabaseSaveModal {
    fn render(&self, frame: &mut Frame, area: Rect, _context: &AppContext, db_manager: &DatabaseManager) {
        // Exact rendering logic from render_save_filename_input
        let popup_width = 60;
        let popup_height = 6;
        let x = (area.width.saturating_sub(popup_width)) / 2;
        let y = (area.height.saturating_sub(popup_height)) / 2;

        let popup_area = Rect {
            x,
            y,
            width: popup_width,
            height: popup_height,
        };

        let current_db = db_manager.get_current_database().unwrap_or("none");
        let display_filename = if self.filename.is_empty() {
            "_"
        } else {
            &self.filename
        };

        let content = format!(
            "Save Database: {}\n\nFilename: {}\n\nPress Enter to save, Esc to cancel",
            current_db, display_filename
        );

        let popup = Paragraph::new(content)
            .block(
                Block::default()
                    .title("Save Database")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            )
            .style(Style::default().fg(Color::White).bg(Color::Black).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center);

        frame.render_widget(popup, popup_area);
    }
    
    fn handle_event(&mut self, event: KeyEvent, context: &mut AppContext, db_manager: &mut DatabaseManager) -> StateTransition {
        // Exact event handling logic from save filename input section in ui/mod.rs
        match event.code {
            KeyCode::Esc => {
                // Equivalent to self.state.cancel_save_filename_input()
                StateTransition::Pop
            }
            KeyCode::Enter => {
                if !self.filename.trim().is_empty() {
                    self.save_database_to_file(context, db_manager);
                    StateTransition::Pop
                } else {
                    StateTransition::Stay
                }
            }
            KeyCode::Backspace => {
                // Equivalent to self.state.remove_char_from_save_filename()
                self.filename.pop();
                StateTransition::Stay
            }
            KeyCode::Char(c) => {
                // Equivalent to self.state.add_char_to_save_filename(c)
                self.filename.push(c);
                StateTransition::Stay
            }
            _ => StateTransition::Stay,
        }
    }
    
    fn name(&self) -> &'static str {
        "DatabaseSaveModal"
    }
}