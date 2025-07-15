use anyhow::Result;
use crossterm::event::KeyEvent;
use ratatui::{Frame, layout::{Rect, Layout, Direction, Constraint}};
use std::collections::HashMap;

use super::{UIState, StateTransition, StateContext, PanelType};

/// StateManager manages multiple panels and coordinates state transitions
pub struct StateManager {
    panels: HashMap<PanelType, Box<dyn UIState>>,
    modal_stack: Vec<Box<dyn UIState>>, // For modal states (overlays)
    focused_panel: PanelType,
    context: StateContext,
}

impl StateManager {
    pub fn new(context: StateContext) -> Self {
        let mut panels = HashMap::new();
        
        // Initialize all panels
        panels.insert(PanelType::Database, Box::new(super::states::DatabaseSelectPanel::new()) as Box<dyn UIState>);
        panels.insert(PanelType::Table, Box::new(super::states::TableSelectPanel::new()) as Box<dyn UIState>);
        panels.insert(PanelType::Main, Box::new(super::states::TableDataViewer::new()) as Box<dyn UIState>);
        panels.insert(PanelType::Status, Box::new(super::states::StatusBar::new()) as Box<dyn UIState>);
        
        Self {
            panels,
            modal_stack: Vec::new(),
            focused_panel: PanelType::Database, // Start with database panel focused
            context,
        }
    }
    
    /// Get the currently focused panel
    pub fn get_focused_panel(&self) -> PanelType {
        self.focused_panel
    }
    
    /// Get a reference to a specific panel
    pub fn get_panel(&self, panel_type: PanelType) -> Option<&dyn UIState> {
        self.panels.get(&panel_type).map(|s| s.as_ref())
    }
    
    /// Handle a key event by passing it to the focused panel or modal
    pub fn handle_event(&mut self, event: KeyEvent) -> Result<bool> {
        // If there's a modal open, handle it first
        if let Some(modal) = self.modal_stack.last_mut() {
            let state_name = modal.debug_name();
            self.context.action_logger.log_debug(&format!("Handling event in modal: {}", state_name));
            
            let transition = modal.handle_event(event, &mut self.context);
            self.handle_transition(transition)?;
        } else {
            // Handle event in focused panel
            let state_name = self.panels.get(&self.focused_panel)
                .map(|s| s.debug_name())
                .unwrap_or("Unknown");
            
            self.context.action_logger.log_debug(&format!("Handling event in panel: {} ({})", state_name, format!("{:?}", self.focused_panel)));
            
            let transition = if let Some(panel) = self.panels.get_mut(&self.focused_panel) {
                panel.handle_event(event, &mut self.context)
            } else {
                StateTransition::Stay
            };
            
            self.handle_transition(transition)?;
        }
        
        // Return false only if we should exit the application
        Ok(!self.panels.is_empty())
    }
    
    /// Handle a state transition
    fn handle_transition(&mut self, transition: StateTransition) -> Result<()> {
        match transition {
            StateTransition::Stay => {
                // Do nothing
            }
            StateTransition::Push(mut new_state) => {
                let new_state_name = new_state.debug_name();
                self.context.action_logger.log_debug(&format!("Pushing modal: {}", new_state_name));
                
                new_state.on_enter(&mut self.context)?;
                self.modal_stack.push(new_state);
            }
            StateTransition::Pop => {
                if let Some(mut old_modal) = self.modal_stack.pop() {
                    old_modal.on_exit(&mut self.context)?;
                    self.context.action_logger.log_debug(&format!("Popped modal: {}", old_modal.debug_name()));
                }
            }
            StateTransition::Replace(mut new_state) => {
                // Replace the current focused panel with new state
                let old_panel_name = self.panels.get(&self.focused_panel)
                    .map(|s| s.debug_name())
                    .unwrap_or("Unknown");
                
                if let Some(mut old_state) = self.panels.remove(&self.focused_panel) {
                    old_state.on_exit(&mut self.context)?;
                }
                
                let new_state_name = new_state.debug_name();
                self.context.action_logger.log_debug(&format!(
                    "Replacing panel {:?}: {} -> {}",
                    self.focused_panel,
                    old_panel_name,
                    new_state_name
                ));
                
                new_state.on_enter(&mut self.context)?;
                self.panels.insert(self.focused_panel, new_state);
            }
            StateTransition::FocusPanel(panel_type) => {
                self.context.action_logger.log_debug(&format!(
                    "Focusing panel: {:?} -> {:?}",
                    self.focused_panel,
                    panel_type
                ));
                self.focused_panel = panel_type;
            }
            StateTransition::Exit => {
                self.context.action_logger.log_info("Application exit requested");
                // Clear all panels to signal exit
                for (_, mut panel) in self.panels.drain() {
                    panel.on_exit(&mut self.context)?;
                }
                // Clear modals too
                while let Some(mut modal) = self.modal_stack.pop() {
                    modal.on_exit(&mut self.context)?;
                }
            }
        }
        Ok(())
    }
    
    /// Render all panels in their designated layout areas
    pub fn render(&self, frame: &mut Frame, area: Rect) -> Result<()> {
        // Create the main layout: Header, Content, Status
        let main_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(0),    // Main content area
                Constraint::Length(3), // Status bar
            ])
            .split(area);

        // Render header
        self.render_header(frame, main_layout[0])?;

        // Split main content area: Left sidebar (30%), Main content (70%)
        let content_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(30), // Left sidebar (database + table list)
                Constraint::Percentage(70), // Main content panel
            ])
            .split(main_layout[1]);

        // Split left sidebar: Database panel (40%), Table panel (60%)
        let left_sidebar_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(40), // Database panel
                Constraint::Percentage(60), // Table panel
            ])
            .split(content_layout[0]);

        // Render each panel in its area
        if let Some(db_panel) = self.panels.get(&PanelType::Database) {
            let is_focused = self.focused_panel == PanelType::Database && self.modal_stack.is_empty();
            db_panel.render(frame, left_sidebar_layout[0], is_focused, &self.context)?;
        }

        if let Some(table_panel) = self.panels.get(&PanelType::Table) {
            let is_focused = self.focused_panel == PanelType::Table && self.modal_stack.is_empty();
            table_panel.render(frame, left_sidebar_layout[1], is_focused, &self.context)?;
        }

        if let Some(main_panel) = self.panels.get(&PanelType::Main) {
            let is_focused = self.focused_panel == PanelType::Main && self.modal_stack.is_empty();
            main_panel.render(frame, content_layout[1], is_focused, &self.context)?;
        }

        // Render status bar
        if let Some(status_panel) = self.panels.get(&PanelType::Status) {
            let is_focused = self.focused_panel == PanelType::Status && self.modal_stack.is_empty();
            status_panel.render(frame, main_layout[2], is_focused, &self.context)?;
        }

        // Render any modals on top
        for modal in &self.modal_stack {
            modal.render(frame, area, true, &self.context)?;
        }

        Ok(())
    }

    /// Render the application header
    fn render_header(&self, frame: &mut Frame, area: Rect) -> Result<()> {
        use ratatui::{
            widgets::{Block, Borders, Paragraph},
            style::{Style, Color, Modifier},
            layout::Alignment,
        };

        let header = Paragraph::new("🦆 Ducky - DuckDB TUI")
            .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        
        frame.render_widget(header, area);
        Ok(())
    }
    
    /// Update notifications (remove expired ones)
    pub fn update_notifications(&mut self) {
        self.context.update_notifications();
    }
    
    /// Get read-only access to the context for external access to shared data
    pub fn context(&self) -> &StateContext {
        &self.context
    }
    
    /// Get mutable access to the context for external modification
    pub fn context_mut(&mut self) -> &mut StateContext {
        &mut self.context
    }
}