use ratatui::prelude::*;
use crossterm::event::KeyEvent;
use crate::state::context::AppContext;
use crate::db::DatabaseManager;

pub trait UIState {
    fn render(&self, frame: &mut Frame, area: Rect, context: &AppContext, db_manager: &DatabaseManager);
    fn handle_event(&mut self, event: KeyEvent, context: &mut AppContext, db_manager: &mut DatabaseManager) -> StateTransition;
    fn name(&self) -> &'static str;
    
    // Panel lifecycle methods with default no-op implementations
    fn on_enter(&mut self, _context: &mut AppContext, _db_manager: &mut DatabaseManager) {}
    fn on_exit(&mut self, _context: &mut AppContext, _db_manager: &mut DatabaseManager) {}
    
    // Sync method for keeping state in sync with context changes - default no-op
    fn sync(&mut self, _context: &AppContext, _db_manager: &DatabaseManager) {}
}

#[derive(Debug, Clone, PartialEq)]
pub enum StateTransition {
    Stay,
    Exit,
    To(StateKey),
    Push(ModalKey),         // Push a modal onto the stack
    PushState(StateKey),    // Push a state onto the stack (e.g., filter input)
    Pop,                    // Pop the current modal/state from the stack
}

#[derive(Debug, Clone, PartialEq)]
pub enum StateKey {
    DatabaseSelect,
    TableSelect,
    TableDataViewer,
    TableInspector,
    ColumnFilterInput(String), // Column name being filtered
}

#[derive(Debug, Clone, PartialEq)]
pub enum ModalKey {
    DeleteConfirmation(crate::state::states::DeleteTarget),
    DatabaseNameInput,
    DatabaseSave,
}