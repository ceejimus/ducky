use anyhow::Result;
use crossterm::event::KeyEvent;
use ratatui::{Frame, layout::Rect};

pub mod context;
pub mod manager;
pub mod states;

pub use context::StateContext;
pub use manager::StateManager;

/// Core trait for all UI states in the application
/// Each state handles its own rendering and input events
pub trait UIState {
    /// Render this state to the given area
    /// is_active indicates if this state should be highlighted/focused
    fn render(&self, frame: &mut Frame, area: Rect, is_active: bool, context: &StateContext) -> Result<()>;
    
    /// Handle a key event and return the resulting state transition
    fn handle_event(&mut self, event: KeyEvent, context: &mut StateContext) -> StateTransition;
    
    /// Called when transitioning into this state
    fn on_enter(&mut self, context: &mut StateContext) -> Result<()> {
        let _ = context; // Default implementation does nothing
        Ok(())
    }
    
    /// Called when transitioning out of this state
    fn on_exit(&mut self, context: &mut StateContext) -> Result<()> {
        let _ = context; // Default implementation does nothing
        Ok(())
    }
    
    /// Get a debug name for this state (for logging)
    fn debug_name(&self) -> &'static str;
}

/// Represents different panels in the multi-panel layout
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PanelType {
    Database,
    Table,
    Main,
    Status,
}

/// Represents possible state transitions
pub enum StateTransition {
    /// Stay in the current state (no transition)
    Stay,
    /// Push a new state onto the stack (modal/overlay behavior)  
    Push(Box<dyn UIState>),
    /// Pop the current state from the stack (return to previous)
    Pop,
    /// Replace the current state with a new one
    Replace(Box<dyn UIState>),
    /// Focus a specific panel (for multi-panel navigation)
    FocusPanel(PanelType),
    /// Exit the application
    Exit,
}

/// Shared data that needs to be accessible across multiple states
#[derive(Debug, Clone)]
pub struct GlobalState {
    pub selected_database: Option<String>,
    pub selected_table: Option<String>,
    pub status_message: String,
}

impl Default for GlobalState {
    fn default() -> Self {
        Self {
            selected_database: None,
            selected_table: None,
            status_message: "Ready".to_string(),
        }
    }
}