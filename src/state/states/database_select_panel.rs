use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem},
};

use crate::state::{UIState, StateTransition, StateContext};
use crate::app::state::Notification;

/// DatabaseSelectPanel manages the database list and selection
pub struct DatabaseSelectPanel {
    selected_index: usize,
    dropdown_expanded: bool,
}

impl DatabaseSelectPanel {
    pub fn new() -> Self {
        Self {
            selected_index: 0,
            dropdown_expanded: false,
        }
    }
    
    /// Sync the selected index with the current database
    fn sync_selected_index(&mut self, context: &StateContext) {
        if let Some(current_db) = &context.global_state.selected_database {
            let databases = context.database_manager.get_databases();
            if let Some(index) = databases.iter().position(|db| &db.name == current_db) {
                self.selected_index = index;
            }
        }
    }
    
    /// Update the selected database in the context
    fn update_selected_database(&self, context: &mut StateContext) {
        // Clone the database info to avoid borrowing issues
        let db_info = {
            let databases = context.database_manager.get_databases();
            databases.get(self.selected_index).map(|db| (db.name.clone(), db.is_memory))
        };
        
        if let Some((db_name, _is_memory)) = db_info {
            context.set_selected_database(Some(db_name.clone()));
            
            // Set the current database in the manager
            if let Err(e) = context.database_manager.set_current_database(&db_name) {
                context.action_logger.log_error(&format!("Failed to set current database: {e}"));
                context.add_notification(Notification::error(format!("Failed to switch to database: {e}")));
            } else {
                context.action_logger.log_info(&format!("Switched to database: {}", db_name));
                context.set_status_message(format!("Database: {}", db_name));
            }
        }
    }
}

impl UIState for DatabaseSelectPanel {
    fn render(&mut self, frame: &mut Frame, area: Rect, is_active: bool, context: &mut StateContext) -> Result<()> {
        let databases = context.database_manager.get_databases();
        
        let items: Vec<ListItem> = databases
            .iter()
            .enumerate()
            .map(|(i, db)| {
                let style = if i == self.selected_index {
                    if is_active {
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
                    }
                } else {
                    Style::default().fg(Color::White)
                };
                
                let display_text = if db.is_memory {
                    format!("📄 {}", db.name)
                } else {
                    format!("🗃️  {}", db.name)
                };
                
                ListItem::new(display_text).style(style)
            })
            .collect();
        
        let border_style = if is_active {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };
        
        let list = List::new(items)
            .block(
                Block::default()
                    .title("Databases")
                    .borders(Borders::ALL)
                    .border_style(border_style)
            )
            .style(Style::default().fg(Color::White));
        
        frame.render_widget(list, area);
        Ok(())
    }
    
    fn handle_event(&mut self, event: KeyEvent, context: &mut StateContext) -> StateTransition {
        match event.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                    self.update_selected_database(context);
                }
                StateTransition::Stay
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let databases = context.database_manager.get_databases();
                if self.selected_index < databases.len().saturating_sub(1) {
                    self.selected_index += 1;
                    self.update_selected_database(context);
                }
                StateTransition::Stay
            }
            KeyCode::Enter => {
                self.update_selected_database(context);
                // Switch to the table panel within the left sidebar
                StateTransition::SwitchLeftSidebarPanel
            }
            KeyCode::Left | KeyCode::Char('h') | KeyCode::Right | KeyCode::Char('l') => {
                // Switch to the table panel (horizontal navigation)
                StateTransition::SwitchLeftSidebarPanel
            }
            KeyCode::Char('n') => {
                // Create new database - push modal state
                StateTransition::Push(Box::new(super::TableNameInput::new_for_database()))
            }
            KeyCode::Char('o') => {
                // Open file browser for database connection
                // TODO: Implement file browser modal
                context.action_logger.log_info("File browser not yet implemented");
                context.add_notification(Notification::info("File browser coming soon".to_string()));
                StateTransition::Stay
            }
            KeyCode::Char('s') => {
                // Save database to file (only for in-memory databases)
                let databases = context.database_manager.get_databases();
                if let Some(db) = databases.get(self.selected_index) {
                    if db.is_memory {
                        // TODO: Implement save filename input modal
                        context.action_logger.log_info(&format!("Save database {} to file", db.name));
                        context.add_notification(Notification::info("Save functionality coming soon".to_string()));
                    } else {
                        context.add_notification(Notification::info("Database already saved to disk".to_string()));
                    }
                }
                StateTransition::Stay
            }
            KeyCode::Delete | KeyCode::Char('d') => {
                let databases = context.database_manager.get_databases();
                if let Some(db) = databases.get(self.selected_index) {
                    // Push delete confirmation modal
                    StateTransition::Push(Box::new(super::DeleteConfirmation::new_for_database(db.name.clone())))
                } else {
                    StateTransition::Stay
                }
            }
            _ => StateTransition::Stay,
        }
    }
    
    fn on_enter(&mut self, context: &mut StateContext) -> Result<()> {
        context.action_logger.log_debug("Entered DatabaseSelectPanel");
        self.sync_selected_index(context);
        self.update_selected_database(context);
        Ok(())
    }
    
    fn debug_name(&self) -> &'static str {
        "DatabaseSelectPanel"
    }
}

impl Default for DatabaseSelectPanel {
    fn default() -> Self {
        Self::new()
    }
}