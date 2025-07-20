use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Rect, Layout, Direction, Constraint},
    style::{Color, Style, Modifier},
    widgets::{Block, Borders, Paragraph, List, ListItem},
};

use crate::state::{UIState, StateTransition, StateContext};

/// ColumnReorderMode handles column reordering and visibility sub-state
pub struct ColumnReorderMode {
    selected_column_index: usize,
    column_order: Vec<usize>,
    hidden_columns: std::collections::HashSet<usize>,
    original_column_order: Vec<usize>,
    original_hidden_columns: std::collections::HashSet<usize>,
    scroll_offset: usize,
}

impl ColumnReorderMode {
    pub fn new() -> Self {
        Self {
            selected_column_index: 0,
            column_order: Vec::new(),
            hidden_columns: std::collections::HashSet::new(),
            original_column_order: Vec::new(),
            original_hidden_columns: std::collections::HashSet::new(),
            scroll_offset: 0,
        }
    }
    
    /// Initialize the column state from the table data and existing global state
    fn initialize_from_table_data(&mut self, context: &StateContext) {
        if let Some(table_name) = &context.global_state.selected_table {
            if let Some(current_db) = context.database_manager.get_current_database() {
                if let Some(connection) = context.database_manager.get_connection(current_db) {
                    // Get all column names from table
                    let sql = format!("SELECT * FROM {} LIMIT 1", table_name);
                    if let Ok(mut stmt) = connection.prepare(&sql) {
                        if let Ok(rows) = stmt.query([]) {
                            let column_count = rows.as_ref().unwrap().column_count();
                            let mut all_column_names = Vec::new();
                            
                            // Get all column names
                            for i in 0..column_count {
                                if let Ok(name) = rows.as_ref().unwrap().column_name(i) {
                                    all_column_names.push(name.to_string());
                                }
                            }
                            
                            // Check if we have existing custom order for this table
                            if let Some(existing_order) = context.global_state.column_order.get(table_name) {
                                // Use existing custom order
                                let mut ordered_indices = Vec::new();
                                for col_name in existing_order {
                                    if let Some(idx) = all_column_names.iter().position(|name| name == col_name) {
                                        ordered_indices.push(idx);
                                    }
                                }
                                
                                // Add any columns not in the existing order (new columns)
                                for (idx, _) in all_column_names.iter().enumerate() {
                                    if !ordered_indices.contains(&idx) {
                                        ordered_indices.push(idx);
                                    }
                                }
                                
                                self.column_order = ordered_indices;
                            } else {
                                // No existing order, use default order
                                self.column_order = (0..column_count).collect();
                            }
                            
                            // Load existing hidden columns
                            if let Some(existing_hidden) = context.global_state.hidden_columns.get(table_name) {
                                for col_name in existing_hidden {
                                    if let Some(idx) = all_column_names.iter().position(|name| name == col_name) {
                                        self.hidden_columns.insert(idx);
                                    }
                                }
                            }
                            
                            // Store original state for cancel operation
                            self.original_column_order = self.column_order.clone();
                            self.original_hidden_columns = self.hidden_columns.clone();
                            
                            // Set selected column index to match the currently selected column
                            if let Some(ref selected_col_name) = context.global_state.current_selected_column {
                                if let Some(original_idx) = all_column_names.iter().position(|name| name == selected_col_name) {
                                    // Find this original index in our column_order to get the display position
                                    if let Some(display_pos) = self.column_order.iter().position(|&idx| idx == original_idx) {
                                        self.selected_column_index = display_pos;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    /// Get the currently selected column's original index
    fn get_selected_original_index(&self) -> Option<usize> {
        self.column_order.get(self.selected_column_index).copied()
    }
    
    /// Move the selected column left in the order
    fn move_column_left(&mut self) {
        if self.selected_column_index > 0 {
            self.column_order.swap(self.selected_column_index, self.selected_column_index - 1);
            self.selected_column_index -= 1;
        }
    }
    
    /// Move the selected column right in the order
    fn move_column_right(&mut self) {
        if self.selected_column_index < self.column_order.len() - 1 {
            self.column_order.swap(self.selected_column_index, self.selected_column_index + 1);
            self.selected_column_index += 1;
        }
    }
    
    /// Move the selected column to the beginning
    fn move_column_to_start(&mut self) {
        if self.selected_column_index > 0 {
            let column_id = self.column_order.remove(self.selected_column_index);
            self.column_order.insert(0, column_id);
            self.selected_column_index = 0;
        }
    }
    
    /// Move the selected column to the end
    fn move_column_to_end(&mut self) {
        if self.selected_column_index < self.column_order.len() - 1 {
            let column_id = self.column_order.remove(self.selected_column_index);
            self.column_order.push(column_id);
            self.selected_column_index = self.column_order.len() - 1;
        }
    }
    
    /// Toggle visibility of the selected column
    fn toggle_column_visibility(&mut self) {
        if let Some(original_index) = self.get_selected_original_index() {
            if self.hidden_columns.contains(&original_index) {
                self.hidden_columns.remove(&original_index);
            } else {
                self.hidden_columns.insert(original_index);
            }
        }
    }
    
    /// Restore original state (cancel changes)
    fn restore_original_state(&mut self) {
        self.column_order = self.original_column_order.clone();
        self.hidden_columns = self.original_hidden_columns.clone();
    }
    
    /// Update scroll offset to ensure selected item is visible
    fn ensure_selected_visible(&mut self, visible_area_height: usize) {
        if visible_area_height == 0 {
            return;
        }
        
        // Ensure selected item is within the visible area
        if self.selected_column_index < self.scroll_offset {
            self.scroll_offset = self.selected_column_index;
        } else if self.selected_column_index >= self.scroll_offset + visible_area_height {
            self.scroll_offset = self.selected_column_index.saturating_sub(visible_area_height - 1);
        }
    }
    
    /// Apply column order and visibility changes to global state
    fn apply_changes(&self, context: &mut StateContext) {
        let table_name = if let Some(name) = &context.global_state.selected_table {
            name.clone()
        } else {
            return;
        };
        
        // Get column names in the new order
        let mut ordered_column_names = Vec::new();
        let mut hidden_column_names = std::collections::HashSet::new();
        let mut currently_selected_column_name = None;
        
        if let Some(current_db) = context.database_manager.get_current_database() {
            if let Some(connection) = context.database_manager.get_connection(current_db) {
                let sql = format!("SELECT * FROM {} LIMIT 1", table_name);
                if let Ok(mut stmt) = connection.prepare(&sql) {
                    if let Ok(rows) = stmt.query([]) {
                        let column_count = rows.as_ref().unwrap().column_count();
                        let mut all_column_names = Vec::new();
                        
                        // Get all column names first
                        for i in 0..column_count {
                            if let Ok(name) = rows.as_ref().unwrap().column_name(i) {
                                all_column_names.push(name.to_string());
                            }
                        }
                        
                        // Build ordered column names based on column_order
                        for (display_pos, &original_index) in self.column_order.iter().enumerate() {
                            if let Some(name) = all_column_names.get(original_index) {
                                ordered_column_names.push(name.clone());
                                
                                // Preserve the currently selected column
                                if display_pos == self.selected_column_index {
                                    currently_selected_column_name = Some(name.clone());
                                }
                                
                                // Check if this column is hidden
                                if self.hidden_columns.contains(&original_index) {
                                    hidden_column_names.insert(name.clone());
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Save to global state using the new methods
        context.global_state.set_column_order(table_name.clone(), ordered_column_names);
        context.global_state.set_hidden_columns(table_name, hidden_column_names);
        
        // Preserve the currently selected column in global state
        context.global_state.current_selected_column = currently_selected_column_name;
    }
}

impl UIState for ColumnReorderMode {
    fn render(&mut self, frame: &mut Frame, area: Rect, is_active: bool, context: &mut StateContext) -> Result<()> {
        let border_style = if is_active {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };
        
        // Split the area for instructions and column list
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4), // Instructions
                Constraint::Min(0),    // Column list
            ])
            .split(area);
        
        // Render instructions
        let instructions = Paragraph::new(
            "h/l: Move column left/right  H/L: Move to start/end\n\
             j/k: Select column  o: Toggle visibility\n\
             Enter: Apply changes  Esc: Cancel"
        )
        .block(
            Block::default()
                .title("Column Modify Mode")
                .borders(Borders::ALL)
                .border_style(border_style)
        )
        .style(Style::default().fg(Color::White));
        
        frame.render_widget(instructions, layout[0]);
        
        // Get column names from current table
        let mut column_names = Vec::new();
        if let Some(table_name) = &context.global_state.selected_table {
            if let Some(current_db) = context.database_manager.get_current_database() {
                if let Some(connection) = context.database_manager.get_connection(current_db) {
                    let sql = format!("SELECT * FROM {} LIMIT 1", table_name);
                    if let Ok(mut stmt) = connection.prepare(&sql) {
                        if let Ok(rows) = stmt.query([]) {
                            let column_count = rows.as_ref().unwrap().column_count();
                            for i in 0..column_count {
                                if let Ok(name) = rows.as_ref().unwrap().column_name(i) {
                                    column_names.push(name.to_string());
                                } else {
                                    column_names.push(format!("Column_{}", i + 1));
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Calculate available height for the list (subtract borders)
        let available_height = layout[1].height.saturating_sub(2) as usize;
        
        // Update scroll offset to ensure selected item is visible
        self.ensure_selected_visible(available_height);
        
        // Create list items for columns in their current order, only for visible items
        let visible_end = (self.scroll_offset + available_height).min(self.column_order.len());
        let items: Vec<ListItem> = self.column_order
            .iter()
            .enumerate()
            .skip(self.scroll_offset)
            .take(available_height)
            .filter_map(|(display_pos, &original_col_index)| {
                column_names.get(original_col_index).map(|name| {
                    let is_selected = display_pos == self.selected_column_index;
                    let is_hidden = self.hidden_columns.contains(&original_col_index);
                    
                    let style = if is_selected {
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                    } else if is_hidden {
                        Style::default().fg(Color::DarkGray)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    
                    let text = if is_hidden {
                        format!("{}: {} (hidden)", display_pos + 1, name)
                    } else {
                        format!("{}: {}", display_pos + 1, name)
                    };
                    
                    ListItem::new(text).style(style)
                })
            })
            .collect();
        
        // Create title with scroll information if needed
        let title = if self.column_order.len() > available_height {
            format!("Column Order ({}/{}) [{}-{}]", 
                    self.selected_column_index + 1, 
                    self.column_order.len(),
                    self.scroll_offset + 1,
                    visible_end)
        } else {
            format!("Column Order ({}/{})", 
                    self.selected_column_index + 1, 
                    self.column_order.len())
        };
        
        let list = List::new(items)
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(border_style)
            )
            .style(Style::default().fg(Color::White));
        
        frame.render_widget(list, layout[1]);
        Ok(())
    }
    
    fn handle_event(&mut self, event: KeyEvent, context: &mut StateContext) -> StateTransition {
        match event.code {
            KeyCode::Esc => {
                // Cancel changes and restore original state
                self.restore_original_state();
                context.action_logger.log_info("Column modifications cancelled");
                StateTransition::Pop
            }
            KeyCode::Enter => {
                // Apply changes and exit modify mode
                self.apply_changes(context);
                context.action_logger.log_info("Column modifications applied");
                // Set a flag to trigger data refresh when returning to table viewer
                context.set_status_message("Column order updated - refreshing data".to_string());
                StateTransition::Pop
            }
            KeyCode::Char('h') => {
                // Move column left
                self.move_column_left();
                StateTransition::Stay
            }
            KeyCode::Char('l') => {
                // Move column right
                self.move_column_right();
                StateTransition::Stay
            }
            KeyCode::Char('H') => {
                // Move column to start
                self.move_column_to_start();
                StateTransition::Stay
            }
            KeyCode::Char('L') => {
                // Move column to end
                self.move_column_to_end();
                StateTransition::Stay
            }
            KeyCode::Char('j') | KeyCode::Down => {
                // Select next column
                if !self.column_order.is_empty() && self.selected_column_index < self.column_order.len() - 1 {
                    self.selected_column_index += 1;
                }
                StateTransition::Stay
            }
            KeyCode::Char('k') | KeyCode::Up => {
                // Select previous column
                if self.selected_column_index > 0 {
                    self.selected_column_index -= 1;
                }
                StateTransition::Stay
            }
            KeyCode::Char('o') => {
                // Toggle column visibility
                self.toggle_column_visibility();
                StateTransition::Stay
            }
            _ => StateTransition::Stay,
        }
    }
    
    fn on_enter(&mut self, context: &mut StateContext) -> Result<()> {
        context.action_logger.log_debug("Entered ColumnReorderMode");
        self.initialize_from_table_data(context);
        Ok(())
    }
    
    fn debug_name(&self) -> &'static str {
        "ColumnReorderMode"
    }
}

impl Default for ColumnReorderMode {
    fn default() -> Self {
        Self::new()
    }
}