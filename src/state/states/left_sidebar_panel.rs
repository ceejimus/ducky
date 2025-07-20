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
        // Pass the event to the active sub-state and handle any panel switching they request
        let transition = match self.active_panel {
            LeftPanelSelection::Database => self.database_panel.handle_event(event, context),
            LeftPanelSelection::Table => self.table_panel.handle_event(event, context),
        };
        
        // Check if the sub-state wants to switch panels or do other transitions
        match transition {
            StateTransition::SwitchLeftSidebarPanel => {
                // Switch to the other panel within the sidebar
                match self.active_panel {
                    LeftPanelSelection::Database => self.focus_table_panel(),
                    LeftPanelSelection::Table => self.focus_database_panel(),
                }
                StateTransition::Stay
            }
            StateTransition::Stay => StateTransition::Stay,
            other => other, // Pass through all other transitions (modals, focus changes, etc.)
        }
    }
    
    fn on_enter(&mut self, context: &mut StateContext) -> Result<()> {
        context.action_logger.log_debug("Entered LeftSidebarPanel");
        // Initialize both panels
        self.database_panel.on_enter(context)?;
        self.table_panel.on_enter(context)?;
        
        // If a CLI database was provided, auto-focus the table panel
        if context.global_state.cli_database_provided {
            self.focus_table_panel();
            context.action_logger.log_info("Auto-focused table panel due to CLI database");
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