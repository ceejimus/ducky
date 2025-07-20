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
    fn render(&mut self, frame: &mut Frame, area: Rect, is_active: bool, context: &mut StateContext) -> Result<()>;
    
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
    
    /// Called when a modal completion requires special handling
    fn handle_modal_completion(&mut self, modal_name: &str, context: &mut StateContext) -> Result<()> {
        let _ = (modal_name, context); // Default implementation does nothing
        Ok(())
    }
    
    /// Get a debug name for this state (for logging)
    fn debug_name(&self) -> &'static str;
}

/// Represents different panels in the multi-panel layout
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PanelType {
    LeftSidebar,
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
    /// Switch to the other sub-panel within the left sidebar (internal navigation)
    SwitchLeftSidebarPanel,
    /// Exit the application
    Exit,
}

/// Direction for sorting columns
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    Ascending,
    Descending,
}

/// Specification for a column in the sort chain
#[derive(Debug, Clone)]
pub struct SortColumnSpec {
    pub column_name: String,
    pub direction: SortDirection,
}

/// Shared data that needs to be accessible across multiple states
#[derive(Debug, Clone)]
pub struct GlobalState {
    pub selected_database: Option<String>,
    pub selected_table: Option<String>,
    pub status_message: String,
    pub cli_database_provided: bool,
    
    // Multi-column sorting state
    pub sort_columns: Vec<SortColumnSpec>,
    
    // Column filtering state  
    pub column_filters: std::collections::HashMap<String, String>, // column_name -> filter_text
    pub search_column: Option<String>, // Currently selected column for search
    pub search_text: String,
    pub is_searching: bool,
    pub search_debounce_timer: Option<std::time::Instant>,
    
    // View creation state
    pub is_entering_view_name: bool,
    pub new_view_name: String,
    
    // Current selected column (for column reorder initialization)
    pub current_selected_column: Option<String>,
    
    // Column management state
    pub column_order: std::collections::HashMap<String, Vec<String>>, // table_name -> ordered_column_names
    pub hidden_columns: std::collections::HashMap<String, std::collections::HashSet<String>>, // table_name -> hidden_column_names
    
}

impl Default for GlobalState {
    fn default() -> Self {
        Self {
            selected_database: None,
            selected_table: None,
            status_message: "Ready".to_string(),
            cli_database_provided: false,
            sort_columns: Vec::new(),
            column_filters: std::collections::HashMap::new(),
            search_column: None,
            search_text: String::new(),
            is_searching: false,
            search_debounce_timer: None,
            is_entering_view_name: false,
            new_view_name: String::new(),
            current_selected_column: None,
            column_order: std::collections::HashMap::new(),
            hidden_columns: std::collections::HashMap::new(),
        }
    }
}

impl GlobalState {
    /// Add or toggle a column in the sort chain
    pub fn toggle_column_sort(&mut self, column_name: String, ascending: bool) {
        let direction = if ascending { SortDirection::Ascending } else { SortDirection::Descending };
        
        // Check if column is already in sort chain
        if let Some(pos) = self.sort_columns.iter().position(|spec| spec.column_name == column_name) {
            let spec = &self.sort_columns[pos];
            if spec.direction == direction {
                // Same direction - remove from chain
                self.sort_columns.remove(pos);
            } else {
                // Different direction - toggle it
                self.sort_columns[pos].direction = direction;
            }
        } else {
            // Not in chain - add it
            self.sort_columns.push(SortColumnSpec {
                column_name,
                direction,
            });
        }
    }
    
    /// Clear all sorting
    pub fn clear_sort(&mut self) {
        self.sort_columns.clear();
    }
    
    /// Check if a column is in the sort chain
    pub fn is_column_sorted(&self, column_name: &str) -> Option<(SortDirection, usize)> {
        self.sort_columns.iter().enumerate()
            .find(|(_, spec)| spec.column_name == column_name)
            .map(|(idx, spec)| (spec.direction, idx + 1))
    }
    
    /// Start column filter input
    pub fn start_column_filter(&mut self, column_name: String) {
        self.search_column = Some(column_name.clone());
        self.search_text = self.column_filters.get(&column_name).cloned().unwrap_or_default();
        self.is_searching = true;
    }
    
    /// Cancel column filter input
    pub fn cancel_filter(&mut self) {
        self.search_column = None;
        self.search_text.clear();
        self.is_searching = false;
        self.search_debounce_timer = None;
    }
    
    /// Apply current filter text to the selected column
    pub fn apply_filter(&mut self) {
        if let Some(column_name) = &self.search_column {
            if self.search_text.is_empty() {
                self.column_filters.remove(column_name);
            } else {
                self.column_filters.insert(column_name.clone(), self.search_text.clone());
            }
        }
        self.cancel_filter();
    }
    
    /// Reset debounce timer on input change
    pub fn reset_debounce_timer(&mut self) {
        self.search_debounce_timer = Some(std::time::Instant::now());
    }
    
    /// Check if enough time has passed for debounced update
    pub fn should_debounce_update(&self) -> bool {
        if let Some(timer) = self.search_debounce_timer {
            timer.elapsed().as_millis() >= 600 // 600ms debounce
        } else {
            false
        }
    }
    
    /// Clear filter on specific column
    pub fn clear_column_filter(&mut self, column_name: &str) {
        self.column_filters.remove(column_name);
    }
    
    /// Clear all filters
    pub fn clear_all_filters(&mut self) {
        self.column_filters.clear();
    }
    
    /// Update column order for a table
    pub fn set_column_order(&mut self, table_name: String, column_names: Vec<String>) {
        self.column_order.insert(table_name, column_names);
    }
    
    /// Update hidden columns for a table
    pub fn set_hidden_columns(&mut self, table_name: String, hidden_columns: std::collections::HashSet<String>) {
        self.hidden_columns.insert(table_name, hidden_columns);
    }
    
    /// Get column order for a table
    pub fn get_column_order(&self, table_name: &str) -> Option<&Vec<String>> {
        self.column_order.get(table_name)
    }
    
    /// Get hidden columns for a table
    pub fn get_hidden_columns(&self, table_name: &str) -> Option<&std::collections::HashSet<String>> {
        self.hidden_columns.get(table_name)
    }
    
    /// Start view name input
    pub fn start_view_name_input(&mut self) {
        self.is_entering_view_name = true;
        self.new_view_name.clear();
    }
    
    /// Cancel view name input
    pub fn cancel_view_name_input(&mut self) {
        self.is_entering_view_name = false;
        self.new_view_name.clear();
    }
    
}