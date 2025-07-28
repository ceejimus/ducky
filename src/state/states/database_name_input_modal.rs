use crate::state::{UIState, StateTransition, AppContext};
use crate::db::DatabaseManager;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

pub struct DatabaseNameInputModal {
    database_name: String,
}

impl DatabaseNameInputModal {
    pub fn new() -> Self {
        Self {
            database_name: String::new(),
        }
    }
    
    fn create_database_with_name(&self, context: &mut AppContext, db_manager: &mut DatabaseManager) {
        let db_name = self.database_name.clone();
        
        // Reuse exact logic from ui/mod.rs create_database_with_name
        if let Err(e) = db_manager.add_database(db_name.clone(), ":memory:".to_string()) {
            context.show_error(format!("Failed to create database: {e}"));
        } else {
            if let Err(e) = db_manager.set_current_database(&db_name) {
                context.show_error(format!("Failed to select database: {e}"));
            } else {
                context.current_database = Some(db_name.clone());
                context.show_success(format!("Created database '{db_name}'"));
            }
        }
    }
}

impl UIState for DatabaseNameInputModal {
    fn render(&self, frame: &mut Frame, area: Rect, _context: &AppContext, _db_manager: &DatabaseManager) {
        // Exact rendering logic from render_database_name_input
        let popup_width = 50;
        let popup_height = 5;
        let x = (area.width.saturating_sub(popup_width)) / 2;
        let y = (area.height.saturating_sub(popup_height)) / 2;

        let popup_area = Rect {
            x,
            y,
            width: popup_width,
            height: popup_height,
        };

        let content = format!(
            "Create New Database\n\nName: {}\n\nPress Enter to create, Esc to cancel",
            if self.database_name.is_empty() {
                "_"
            } else {
                &self.database_name
            }
        );

        let popup = Paragraph::new(content)
            .block(
                Block::default()
                    .title("Database Name")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            )
            .style(Style::default().fg(Color::White).bg(Color::Black).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center);

        frame.render_widget(popup, popup_area);
    }
    
    fn handle_event(&mut self, event: KeyEvent, context: &mut AppContext, db_manager: &mut DatabaseManager) -> StateTransition {
        // Exact event handling logic from database name input section in ui/mod.rs
        match event.code {
            KeyCode::Esc => {
                // Equivalent to self.state.cancel_database_name_input()
                StateTransition::Pop
            }
            KeyCode::Enter => {
                if !self.database_name.trim().is_empty() {
                    self.create_database_with_name(context, db_manager);
                    StateTransition::Pop
                } else {
                    StateTransition::Stay
                }
            }
            KeyCode::Backspace => {
                // Equivalent to self.state.remove_char_from_database_name()
                self.database_name.pop();
                StateTransition::Stay
            }
            KeyCode::Char(c) => {
                // Equivalent to self.state.add_char_to_database_name(c)
                self.database_name.push(c);
                StateTransition::Stay
            }
            _ => StateTransition::Stay,
        }
    }
    
    fn name(&self) -> &'static str {
        "DatabaseNameInputModal"
    }
}