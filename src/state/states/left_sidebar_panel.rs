use anyhow::Result;
use crossterm::event::KeyEvent;
use ratatui::{
    Frame,
    layout::{Rect, Layout, Direction, Constraint},
};

use crate::state::{UIState, StateTransition, StateContext, PanelType};
use super::{DatabaseSelectPanel, TableSelectPanel};


/// Panel selection within the left sidebar
#[derive(Debug, Clone, Copy, PartialEq)]
enum LeftPanelSelection {
    Database,
    Table,
}

/// LeftSidebarPanel manages both Database and Table panels as a unified left sidebar
pub struct LeftSidebarPanel {
    database_panel: DatabaseSelectPanel,
    table_panel: TableSelectPanel,
    active_panel: LeftPanelSelection,
}

impl LeftSidebarPanel {
    pub fn new() -> Self {
        Self {
            database_panel: DatabaseSelectPanel::new(),
            table_panel: TableSelectPanel::new(),
            active_panel: LeftPanelSelection::Database,
        }
    }
    
    /// Switch to database panel
    fn focus_database_panel(&mut self) {
        self.active_panel = LeftPanelSelection::Database;
    }
    
    /// Switch to table panel  
    fn focus_table_panel(&mut self) {
        self.active_panel = LeftPanelSelection::Table;
    }
    
    /// Get the currently active panel type for external reference
    pub fn get_active_panel_type(&self) -> PanelType {
        // The left sidebar is always represented as LeftSidebar externally
        PanelType::LeftSidebar
    }
}

impl UIState for LeftSidebarPanel {
    fn render(&mut self, frame: &mut Frame, area: Rect, is_active: bool, context: &mut StateContext) -> Result<()> {
        // Split left sidebar: Database panel (40%), Table panel (60%)
        let sidebar_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(40), // Database panel
                Constraint::Percentage(60), // Table panel
            ])
            .split(area);

        // Render database panel
        let db_is_focused = is_active && self.active_panel == LeftPanelSelection::Database;
        self.database_panel.render(frame, sidebar_layout[0], db_is_focused, context)?;

        // Render table panel
        let table_is_focused = is_active && self.active_panel == LeftPanelSelection::Table;
        self.table_panel.render(frame, sidebar_layout[1], table_is_focused, context)?;

        Ok(())
    }
    
    fn handle_event(&mut self, event: KeyEvent, context: &mut StateContext) -> StateTransition {
        use crossterm::event::KeyCode;
        
        // Handle panel switching at this level before delegating to sub-panels
        match event.code {
            KeyCode::Left | KeyCode::Char('h') | KeyCode::Right | KeyCode::Char('l') => {
                // Switch to the other panel within the sidebar
                match self.active_panel {
                    LeftPanelSelection::Database => self.focus_table_panel(),
                    LeftPanelSelection::Table => self.focus_database_panel(),
                }
                return StateTransition::Stay;
            }
            _ => {
                // For other events, pass to the active sub-panel
                let transition = match self.active_panel {
                    LeftPanelSelection::Database => self.database_panel.handle_event(event, context),
                    LeftPanelSelection::Table => self.table_panel.handle_event(event, context),
                };
                
                // Handle special case for database panel Enter key - should switch to table panel
                if matches!(event.code, KeyCode::Enter) && matches!(self.active_panel, LeftPanelSelection::Database) {
                    // Database panel selected a database, now switch to table panel
                    self.focus_table_panel();
                    StateTransition::Stay
                } else {
                    // Pass through all other transitions
                    transition
                }
            }
        }
    }
    
    fn on_enter(&mut self, context: &mut StateContext) -> Result<()> {
        tracing::debug!("Entered LeftSidebarPanel");
        // Initialize both panels
        self.database_panel.on_enter(context)?;
        self.table_panel.on_enter(context)?;
        
        // If a CLI database was provided, auto-focus the table panel
        if context.global_state.cli_database_provided {
            self.focus_table_panel();
            tracing::info!("Auto-focused table panel due to CLI database");
        }
        
        Ok(())
    }
    
    fn debug_name(&self) -> &'static str {
        "LeftSidebarPanel"
    }
}

impl Default for LeftSidebarPanel {
    fn default() -> Self {
        Self::new()
    }
}