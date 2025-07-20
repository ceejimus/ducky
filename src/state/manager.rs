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
        panels.insert(PanelType::LeftSidebar, Box::new(super::states::LeftSidebarPanel::new()) as Box<dyn UIState>);
        panels.insert(PanelType::Main, Box::new(super::states::TableDataViewer::new()) as Box<dyn UIState>);
        panels.insert(PanelType::Status, Box::new(super::states::StatusBar::new()) as Box<dyn UIState>);
        
        Self {
            panels,
            modal_stack: Vec::new(),
            focused_panel: PanelType::LeftSidebar, // Start with left sidebar focused
            context,
        }
    }
    
    /// Initialize all panels by calling their on_enter methods
    /// This should be called after the StateManager is created to ensure proper initialization
    pub fn initialize_panels(&mut self) -> Result<()> {
        // Only initialize the focused panel (LeftSidebar) which will handle CLI auto-focus
        // Other panels will be initialized when they are actually focused
        if let Some(panel) = self.panels.get_mut(&self.focused_panel) {
            panel.on_enter(&mut self.context)?;
        }
        
        Ok(())
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
        use crossterm::event::{KeyCode, KeyModifiers};
        
        // Handle global navigation keys first (Tab/Shift+Tab)
        if self.modal_stack.is_empty() {
            match event.code {
                KeyCode::Tab => {
                    if event.modifiers.contains(KeyModifiers::SHIFT) {
                        // Shift+Tab: Move backward between main areas
                        self.focus_previous_panel();
                        return Ok(true);
                    } else {
                        // Tab: Move forward between main areas  
                        self.focus_next_panel();
                        return Ok(true);
                    }
                }
                _ => {}
            }
        }
        
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
    
    /// Focus the next panel in the navigation order
    fn focus_next_panel(&mut self) {
        let next_panel = match self.focused_panel {
            PanelType::LeftSidebar => PanelType::Main,
            PanelType::Main => PanelType::LeftSidebar,
            PanelType::Status => PanelType::LeftSidebar, // Status bar not in normal tab cycle
        };
        
        self.context.action_logger.log_debug(&format!(
            "Tab navigation: {:?} -> {:?}",
            self.focused_panel,
            next_panel
        ));
        
        self.focused_panel = next_panel;
    }
    
    /// Focus the previous panel in the navigation order
    fn focus_previous_panel(&mut self) {
        let prev_panel = match self.focused_panel {
            PanelType::LeftSidebar => PanelType::Main,
            PanelType::Main => PanelType::LeftSidebar,
            PanelType::Status => PanelType::Main,    // Status bar not in normal tab cycle
        };
        
        self.context.action_logger.log_debug(&format!(
            "Shift+Tab navigation: {:?} -> {:?}",
            self.focused_panel,
            prev_panel
        ));
        
        self.focused_panel = prev_panel;
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
                    let modal_name = old_modal.debug_name();
                    old_modal.on_exit(&mut self.context)?;
                    self.context.action_logger.log_debug(&format!("Popped modal: {}", modal_name));
                    
                    // Handle specific modal completions
                    if modal_name == "ColumnReorderMode" {
                        // Column reorder completed - refresh data and adjust selection for hidden columns
                        if let Some(main_panel) = self.panels.get_mut(&PanelType::Main) {
                            main_panel.on_enter(&mut self.context)?;
                            
                            // If this is a TableDataViewer, handle column visibility changes
                            if main_panel.debug_name() == "TableDataViewer" {
                                // Cast to TableDataViewer and handle column reorder completion
                                // Since we can't downcast trait objects easily, we'll use a different approach
                                // Add a method to the UIState trait for this specific case
                                main_panel.handle_modal_completion("ColumnReorderMode", &mut self.context)?;
                            }
                        }
                    } else if modal_name == "ColumnFilterInput" {
                        // Column filter completed - just refresh data normally
                        if let Some(main_panel) = self.panels.get_mut(&PanelType::Main) {
                            main_panel.on_enter(&mut self.context)?;
                        }
                    }
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
                
                let old_panel = self.focused_panel;
                self.focused_panel = panel_type;
                
                // Call on_enter for the newly focused panel if it's different
                if old_panel != panel_type {
                    if let Some(panel) = self.panels.get_mut(&panel_type) {
                        panel.on_enter(&mut self.context)?;
                    }
                }
            }
            StateTransition::SwitchLeftSidebarPanel => {
                // This should only be handled by the LeftSidebarPanel itself, not the StateManager
                self.context.action_logger.log_error("SwitchLeftSidebarPanel transition reached StateManager - this should be handled internally");
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
    pub fn render(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
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

        // Render each panel in its area
        if let Some(left_sidebar) = self.panels.get_mut(&PanelType::LeftSidebar) {
            let is_focused = self.focused_panel == PanelType::LeftSidebar && self.modal_stack.is_empty();
            left_sidebar.render(frame, content_layout[0], is_focused, &mut self.context)?;
        }

        if let Some(main_panel) = self.panels.get_mut(&PanelType::Main) {
            let is_focused = self.focused_panel == PanelType::Main && self.modal_stack.is_empty();
            main_panel.render(frame, content_layout[1], is_focused, &mut self.context)?;
        }

        // Render status bar
        if let Some(status_panel) = self.panels.get_mut(&PanelType::Status) {
            let is_focused = self.focused_panel == PanelType::Status && self.modal_stack.is_empty();
            status_panel.render(frame, main_layout[2], is_focused, &mut self.context)?;
        }

        // Render any modals on top
        for modal in &mut self.modal_stack {
            modal.render(frame, area, true, &mut self.context)?;
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
    
    /// Update notifications (remove expired ones) and check for debounced updates
    pub fn update_notifications(&mut self) {
        self.context.update_notifications();
        
        // Check for debounced filter preview updates
        if let Err(e) = self.check_filter_debounce() {
            self.context.action_logger.log_error(&format!("Filter debounce error: {}", e));
        }
    }
    
    /// Get read-only access to the context for external access to shared data
    pub fn context(&self) -> &StateContext {
        &self.context
    }
    
    /// Get mutable access to the context for external modification
    pub fn context_mut(&mut self) -> &mut StateContext {
        &mut self.context
    }
    
    /// Check for debounced filter preview updates
    pub fn check_filter_debounce(&mut self) -> Result<()> {
        // Only check if ColumnFilterInput modal is active
        if let Some(modal) = self.modal_stack.last() {
            if modal.debug_name() == "ColumnFilterInput" {
                // Check if we should trigger a preview update
                if self.context.global_state.is_searching && 
                   self.context.global_state.should_debounce_update() {
                    
                    // Reset timer to prevent multiple updates
                    self.context.global_state.search_debounce_timer = None;
                    
                    // Only trigger preview if there's search text and it's valid
                    if !self.context.global_state.search_text.trim().is_empty() && 
                       self.is_filter_syntax_valid() {
                        // Trigger preview by temporarily applying filter and refreshing main panel
                        if let (Some(column_name), Some(main_panel)) = 
                            (&self.context.global_state.search_column, self.panels.get_mut(&PanelType::Main)) {
                            
                            // Store current filters
                            let saved_filters = self.context.global_state.column_filters.clone();
                            
                            // Temporarily add search text as filter
                            self.context.global_state.column_filters.insert(
                                column_name.clone(), 
                                self.context.global_state.search_text.trim().to_string()
                            );
                            
                            // Trigger preview update
                            main_panel.on_enter(&mut self.context)?;
                            
                            // Restore original filters (this is just a preview)
                            self.context.global_state.column_filters = saved_filters;
                        }
                    }
                }
            }
        }
        Ok(())
    }
    
    /// Validate filter syntax by testing a query
    fn is_filter_syntax_valid(&self) -> bool {
        // Try to validate by building a test query
        if let (Some(column_name), Some(table_name)) = 
            (&self.context.global_state.search_column, &self.context.global_state.selected_table) {
            
            let test_sql = format!(
                "SELECT COUNT(*) FROM {} WHERE {} {}",
                table_name, column_name, self.context.global_state.search_text.trim()
            );

            // Try to prepare the statement to validate syntax
            if let Some(current_db) = self.context.database_manager.get_current_database() {
                if let Some(connection) = self.context.database_manager.get_connection(current_db) {
                    connection.prepare(&test_sql).is_ok()
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        }
    }
}