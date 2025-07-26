use anyhow::Result;
use crossterm::event::KeyEvent;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};
use std::collections::HashMap;

use super::{PanelType, StateContext, StateTransition, UIState};
use crate::db::query;

/// Direction for panel focus navigation
#[derive(Debug, Copy, Clone)]
enum NavigationDirection {
    Next,
    Previous,
}

/// Focus navigation order - defines the sequence of focusable panels
const FOCUS_ORDER: &[PanelType] = &[PanelType::LeftSidebar, PanelType::Main];

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
        panels.insert(
            PanelType::LeftSidebar,
            Box::new(super::states::LeftSidebarPanel::new()) as Box<dyn UIState>,
        );
        panels.insert(
            PanelType::Main,
            Box::new(super::states::TableDataViewer::new()) as Box<dyn UIState>,
        );
        panels.insert(
            PanelType::Status,
            Box::new(super::states::StatusBar::new()) as Box<dyn UIState>,
        );

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
        if self.modal_stack.is_empty() && event.code == KeyCode::Tab {
            if event.modifiers.contains(KeyModifiers::SHIFT) {
                // Shift+Tab: Move backward between main areas
                self.navigate_panel(NavigationDirection::Previous);
                return Ok(true);
            } else {
                // Tab: Move forward between main areas
                self.navigate_panel(NavigationDirection::Next);
                return Ok(true);
            }
        }

        // If there's a modal open, handle it first
        if let Some(modal) = self.modal_stack.last_mut() {
            let state_name = modal.debug_name();
            tracing::debug!("Handling event in modal: {}", state_name);

            let transition = modal.handle_event(event, &mut self.context);
            self.handle_transition(transition)?;
        } else {
            // Handle event in focused panel
            let state_name = self
                .panels
                .get(&self.focused_panel)
                .map(|s| s.debug_name())
                .unwrap_or("Unknown");

            tracing::debug!(
                "Handling event in panel: {} ({:?})",
                state_name,
                self.focused_panel
            );

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

    /// Navigate focus between panels using modular arithmetic for wrapping
    fn navigate_panel(&mut self, direction: NavigationDirection) {
        // Find current panel index in focus order
        let current_index = FOCUS_ORDER
            .iter()
            .position(|&panel| panel == self.focused_panel)
            .expect("Current focused panel not found in focus order");

        // Calculate next index
        let next_index = match direction {
            NavigationDirection::Next => (current_index + 1) % FOCUS_ORDER.len(),
            NavigationDirection::Previous => {
                if current_index == 0 {
                    FOCUS_ORDER.len() - 1
                } else {
                    current_index - 1
                }
            }
        };

        let next_panel = FOCUS_ORDER[next_index];
        let direction_name = match direction {
            NavigationDirection::Next => "Tab",
            NavigationDirection::Previous => "Shift+Tab",
        };

        tracing::debug!(
            "{} navigation: {:?} -> {:?}",
            direction_name, self.focused_panel, next_panel
        );

        self.focused_panel = next_panel;
    }

    /// Log state transitions with consistent "From -> To" format
    fn log_transition(&mut self, transition_type: &str, from: Option<&str>, to: Option<&str>) {
        let message = match (from, to) {
            (Some(f), Some(t)) => format!("{transition_type}: {f} -> {t}"),
            (Some(f), None) => format!("{transition_type}: {f}"),
            (None, Some(t)) => format!("{transition_type}: -> {t}"),
            (None, None) => transition_type.to_string(),
        };
        tracing::debug!("{}", message);
    }

    /// Handle a state transition
    fn handle_transition(&mut self, transition: StateTransition) -> Result<()> {
        match transition {
            StateTransition::Stay => {
                // Do nothing
            }
            StateTransition::Push(mut new_state) => {
                self.log_transition("Push modal", None, Some(new_state.debug_name()));

                new_state.on_enter(&mut self.context)?;
                self.modal_stack.push(new_state);
            }
            StateTransition::Pop => {
                if let Some(mut old_modal) = self.modal_stack.pop() {
                    old_modal.on_exit(&mut self.context)?;
                    self.log_transition("Pop modal", Some(old_modal.debug_name()), None);

                    // Refresh main panel after modal completion
                    // TableDataViewer will automatically handle any column visibility changes
                    if let Some(main_panel) = self.panels.get_mut(&PanelType::Main) {
                        main_panel.on_enter(&mut self.context)?;
                    }
                }
            }
            StateTransition::Replace(mut new_state) => {
                let mut old_state = self.panels.remove(&self.focused_panel).unwrap();
                old_state.on_exit(&mut self.context)?;

                self.log_transition(
                    "Replace panel",
                    Some(old_state.debug_name()),
                    Some(new_state.debug_name()),
                );

                new_state.on_enter(&mut self.context)?;
                self.panels.insert(self.focused_panel, new_state);
            }
            StateTransition::FocusPanel(panel_type) => {
                self.log_transition(
                    "Focus panel",
                    Some(&format!("{:?}", self.focused_panel)),
                    Some(&format!("{panel_type:?}")),
                );

                let old_panel = self.focused_panel;
                self.focused_panel = panel_type;

                // Call on_enter for the newly focused panel if it's different
                if old_panel != panel_type {
                    if let Some(panel) = self.panels.get_mut(&panel_type) {
                        panel.on_enter(&mut self.context)?;
                    }
                }
            }
            StateTransition::Exit => {
                // Use info for exit since it's more important than debug
                tracing::info!("Exit application");
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
            let is_focused =
                self.focused_panel == PanelType::LeftSidebar && self.modal_stack.is_empty();
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
            layout::Alignment,
            style::{Color, Modifier, Style},
            widgets::{Block, Borders, Paragraph},
        };

        let header = Paragraph::new("🦆 Ducky - DuckDB TUI")
            .style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));

        frame.render_widget(header, area);
        Ok(())
    }

    /// Periodic tick for notifications, debounced updates, and other time-based tasks
    pub fn tick(&mut self) {
        self.context.tick();

        // Check for debounced filter preview updates
        if let Err(e) = self.check_filter_debounce() {
            tracing::error!("Filter debounce error: {}", e);
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
                if self.context.global_state.is_searching
                    && self.context.global_state.should_debounce_update()
                {
                    // Reset timer to prevent multiple updates
                    self.context.global_state.search_debounce_timer = None;

                    // Only trigger preview if there's search text and it's valid
                    if !self.context.global_state.search_text.trim().is_empty()
                        && self.is_filter_syntax_valid()
                    {
                        // Trigger preview by temporarily applying filter and refreshing main panel
                        if let (Some(column_name), Some(main_panel)) = (
                            &self.context.global_state.search_column,
                            self.panels.get_mut(&PanelType::Main),
                        ) {
                            // Store current filters
                            let saved_filters = self.context.global_state.column_filters.clone();

                            // Temporarily add search text as filter
                            self.context.global_state.column_filters.insert(
                                column_name.clone(),
                                self.context.global_state.search_text.trim().to_string(),
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

    /// Validate filter syntax using the db module's validation function
    fn is_filter_syntax_valid(&self) -> bool {
        // Delegate to the database query module for validation
        if let (Some(column_name), Some(table_name)) = (
            &self.context.global_state.search_column,
            &self.context.global_state.selected_table,
        ) {
            if let Some(current_db) = self.context.database_manager.get_current_database() {
                if let Some(connection) = self.context.database_manager.get_connection(current_db) {
                    query::validate_filter_syntax(
                        connection,
                        table_name,
                        column_name,
                        &self.context.global_state.search_text,
                    )
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
