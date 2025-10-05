use crate::db::query::{QueryResult, SortColumnSpec, SortDirection};
use crate::db::DatabaseManager;
use crate::state::{AppContext, StateKey, StateTransition, UIState};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Cell, Row, Table, Paragraph, Wrap, TableState};
use std::collections::{HashMap, HashSet};

// Copy exact enums from ApplicationState
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InspectSection {
    Schema,
    Statistics,
}

impl Default for InspectSection {
    fn default() -> Self {
        Self::Schema
    }
}

// Table creation workflow enum - improved design with data in variants
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TableCreationStep {
    EnteringTableName(String), // Current table name being typed
    SelectingFile(String),     // Table name from previous step
    ImportingData(String),     // Table name from previous step
}

pub struct TableDataViewerState {
    // Table display state - copied exactly from ApplicationState
    pub table_data: Option<QueryResult>,
    pub scroll_x: usize,
    pub scroll_y: usize,
    pub selected_row: usize,
    pub selected_column: Option<String>,
    pub last_table_area_height: u16,

    // Track last viewed table to detect table changes in on_enter
    pub last_viewed_table: Option<String>,

    // Multi-column sorting state - copied exactly from ApplicationState
    pub sort_columns: Vec<SortColumnSpec>,

    // Search/filter state - copied exactly from ApplicationState
    pub is_searching: bool,
    pub search_column: Option<usize>,
    pub search_text: String,
    pub search_syntax_valid: bool,
    pub search_debounce_timer: Option<std::time::Instant>,
    pub column_filters: HashMap<String, String>, // column_name -> filter_text

    // Column expansion state - copied exactly from ApplicationState
    pub expanded_columns: HashSet<usize>,

    // Column ordering state - copied exactly from ApplicationState
    pub column_order: HashMap<String, Vec<String>>, // table_name -> ordered_column_names
    pub original_column_names: Vec<String>, // cached for current table

    // Modal modification state - copied exactly from ApplicationState
    pub is_modifying: bool,
    pub modify_backup_column_order: Option<Vec<String>>, // backup for cancel operation

    // Column hiding state - copied exactly from ApplicationState
    pub hidden_columns: HashMap<String, HashSet<String>>, // table_name -> hidden_column_names

    // Inspect mode state - copied exactly from ApplicationState
    pub inspect_mode: bool,
    pub inspect_active_section: InspectSection,
    pub inspect_schema_scroll_y: usize,
    pub inspect_stats_scroll_y: usize,
    pub inspect_selected_row: usize, // Selected row in the columns view
    // Cache for inspect mode queries (loaded once, not on every render)
    pub inspect_schema_data: Option<QueryResult>,
    pub inspect_stats_data: Option<QueryResult>,

    // Save view input state - copied exactly from ApplicationState
    pub is_entering_view_name: bool,
    pub new_view_name: String,

    // Table creation state - now contained entirely in the Option<TableCreationStep>
    pub table_creation_step: Option<TableCreationStep>,

    // State pattern specific
    pub is_active: bool,
}

impl TableDataViewerState {
    pub fn new() -> Self {
        Self {
            // Initialize exactly like ApplicationState::new()
            table_data: None,
            scroll_x: 0,
            scroll_y: 0,
            selected_row: 0,
            selected_column: None,
            last_table_area_height: 0,
            last_viewed_table: None,
            sort_columns: Vec::new(),
            is_searching: false,
            search_column: None,
            search_text: String::new(),
            search_syntax_valid: true,
            search_debounce_timer: None,
            column_filters: HashMap::new(),
            expanded_columns: HashSet::new(),
            column_order: HashMap::new(),
            original_column_names: Vec::new(),
            is_modifying: false,
            modify_backup_column_order: None,
            hidden_columns: HashMap::new(),
            inspect_mode: false,
            inspect_active_section: InspectSection::Schema,
            inspect_schema_scroll_y: 0,
            inspect_stats_scroll_y: 0,
            inspect_selected_row: 0,
            inspect_schema_data: None,
            inspect_stats_data: None,
            is_entering_view_name: false,
            new_view_name: String::new(),
            table_creation_step: None,
            is_active: false,
        }
    }

    // Placeholder methods for all operations - will be implemented in following phases

    // Filtering methods - placeholders for exact copies from ApplicationState
    pub fn start_column_search(&mut self, _column_index: usize) {
        todo!("Copy exact implementation from ApplicationState::start_column_search")
    }

    pub fn finalize_search(&mut self) -> bool {
        todo!("Copy exact implementation from ApplicationState::finalize_search")
    }

    // Column management methods - placeholders for exact copies from ApplicationState
    pub fn toggle_column_visibility(&mut self) {
        todo!("Copy exact implementation from ApplicationState::toggle_column_visibility")
    }

    pub fn is_column_expanded(&self, column_index: usize) -> bool {
        // Copy exact implementation from ApplicationState::is_column_expanded (lines 629-631)
        self.expanded_columns.contains(&column_index)
    }

    pub fn toggle_column_expansion(&mut self) {
        // Copy exact implementation from ApplicationState::toggle_column_expansion (lines 617-627)
        if let Some(selected_idx) = self.get_selected_column_index() {
            if self.expanded_columns.contains(&selected_idx) {
                // Collapse currently expanded column
                self.expanded_columns.remove(&selected_idx);
            } else {
                // Expand selected column
                self.expanded_columns.insert(selected_idx);
            }
        }
    }

    pub fn get_selected_column_index(&self) -> Option<usize> {
        // Copy exact implementation from ApplicationState::get_selected_column_index (lines 1164-1167)
        if let Some(ref selected_col) = self.selected_column {
            // For now, use a simple implementation without context - will need context for full implementation
            self.original_column_names.iter().position(|name| name == selected_col)
        } else {
            None
        }
    }

    pub fn get_column_index_by_name(&self, name: &str) -> Option<usize> {
        // Copy exact implementation from ApplicationState::get_column_index_by_name (lines 1156-1158)
        // For now, use original column names - will need context for full virtual ordering
        self.original_column_names.iter().position(|col_name| col_name == name)
    }

    // Inspector mode methods - exact copies from ApplicationState (lines 811-862)
    pub fn enter_inspect_mode(&mut self) {
        self.inspect_mode = true;
        self.inspect_active_section = InspectSection::Schema;
        self.inspect_schema_scroll_y = 0;
        self.inspect_stats_scroll_y = 0;
        self.inspect_selected_row = 0;
    }

    pub fn exit_inspect_mode(&mut self) {
        self.inspect_mode = false;
        self.inspect_active_section = InspectSection::Schema;
        self.inspect_schema_scroll_y = 0;
        self.inspect_stats_scroll_y = 0;
        self.inspect_selected_row = 0;
        // Clear cached inspect data
        self.inspect_schema_data = None;
        self.inspect_stats_data = None;
    }

    pub fn inspect_cycle_section(&mut self) {
        self.inspect_active_section = match self.inspect_active_section {
            InspectSection::Schema => InspectSection::Statistics,
            InspectSection::Statistics => InspectSection::Schema,
        };
    }

    pub fn inspect_scroll_up(&mut self) {
        match self.inspect_active_section {
            InspectSection::Schema => {
                if self.inspect_schema_scroll_y > 0 {
                    self.inspect_schema_scroll_y -= 1;
                }
            }
            InspectSection::Statistics => {
                if self.inspect_stats_scroll_y > 0 {
                    self.inspect_stats_scroll_y -= 1;
                }
            }
        }
    }

    pub fn inspect_scroll_down(&mut self, max_rows: usize, visible_rows: usize) {
        match self.inspect_active_section {
            InspectSection::Schema => {
                if self.inspect_schema_scroll_y + visible_rows < max_rows {
                    self.inspect_schema_scroll_y += 1;
                }
            }
            InspectSection::Statistics => {
                if self.inspect_stats_scroll_y + visible_rows < max_rows {
                    self.inspect_stats_scroll_y += 1;
                }
            }
        }
    }

    // Sorting methods - exact copies from ApplicationState (lines 667-720)
    pub fn toggle_in_sort_chain(&mut self, ascending: bool) {
        if let Some(ref column_name) = self.selected_column {
            let desired_direction = if ascending { SortDirection::Ascending } else { SortDirection::Descending };

            // Check if column is already in sort chain
            if let Some(pos) = self.sort_columns.iter().position(|spec| spec.column_name == *column_name) {
                let current_spec = &self.sort_columns[pos];

                if current_spec.direction == desired_direction {
                    // Same direction - remove column from chain
                    self.sort_columns.remove(pos);
                } else {
                    // Different direction - update direction, keep position in chain
                    self.sort_columns[pos].direction = desired_direction;
                }
            } else {
                // Column doesn't exist - add it to end of chain
                self.sort_columns.push(SortColumnSpec {
                    column_name: column_name.clone(),
                    direction: desired_direction,
                });
            }
        }
    }

    // Helper: check if column is in sort chain
    pub fn is_column_in_sort_chain(&self, column_name: &str) -> bool {
        self.sort_columns.iter().any(|spec| spec.column_name == column_name)
    }

    pub fn clear_sort(&mut self) {
        self.sort_columns.clear();
    }

    pub fn get_sort_sql_clause(&self, _column_names: &[String]) -> Option<String> {
        if self.sort_columns.is_empty() {
            return None;
        }

        let mut sort_parts = Vec::new();
        for sort_spec in &self.sort_columns {
            let direction = match sort_spec.direction {
                SortDirection::Ascending => "ASC",
                SortDirection::Descending => "DESC",
            };
            sort_parts.push(format!("{} {}", sort_spec.column_name, direction));
        }

        if sort_parts.is_empty() {
            None
        } else {
            Some(format!("ORDER BY {}", sort_parts.join(", ")))
        }
    }

    // Filtering methods - exact copies from ApplicationState (lines 784-799)
    pub fn get_filter_sql_clause(&self, _column_names: &[String]) -> Option<String> {
        if self.column_filters.is_empty() {
            return None;
        }

        let mut filter_parts = Vec::new();
        for (column_name, filter_text) in &self.column_filters {
            // Use the filter text directly as SQL (user responsibility for syntax)
            filter_parts.push(format!("{column_name} {filter_text}"));
        }

        if filter_parts.is_empty() {
            None
        } else {
            Some(format!("WHERE {}", filter_parts.join(" AND ")))
        }
    }

    pub fn clear_column_filter(&mut self, column_name: &str) {
        self.column_filters.remove(column_name);
    }

    // Data refresh method - re-queries database with current sort/filter settings
    // Exact logic from legacy ui/mod.rs fetch_table_data_preserve_column_with_limit (lines 1133-1189)
    // include_limit parameter matches legacy: true = LIMIT 1000, false = no limit (used after filtering)
    pub fn refresh_table_data(&mut self, context: &mut AppContext, db_manager: &DatabaseManager, include_limit: bool) {
        if let Some(ref table_name) = context.selected_table {
            // Sync filters from context (they may have been updated by filter input state)
            self.column_filters = context.column_filters.clone();

            // Get visible columns (respecting hidden columns and custom order)
            let visible_columns = self.get_visible_column_names(context);

            // Build query spec with current state
            let mut spec = crate::db::query::TableQuerySpec::new(table_name.clone());
            spec.sort_columns = self.sort_columns.clone();
            spec.column_filters = self.column_filters.clone();
            spec.visible_column_names = visible_columns;
            spec.original_column_names = self.original_column_names.clone();
            spec.limit = if include_limit { Some(1000) } else { None }; // Legacy: false = no limit after filtering

            // Execute query
            match db_manager.fetch_table_data(&spec) {
                Ok(new_data) => {
                    // Update table data while preserving selected column position
                    // (exact logic from ApplicationState::update_table_data_preserve_column)
                    let old_selected_column = self.selected_column.clone();

                    self.table_data = Some(new_data.clone());
                    context.table_data = Some(new_data.clone());

                    // Restore selected column if it still exists
                    if let Some(old_col) = old_selected_column {
                        if new_data.columns.contains(&old_col) {
                            self.selected_column = Some(old_col);
                        } else if !new_data.columns.is_empty() {
                            self.selected_column = Some(new_data.columns[0].clone());
                        }
                    }

                    // Ensure selected row is within bounds
                    if !new_data.rows.is_empty() && self.selected_row >= new_data.rows.len() {
                        self.selected_row = new_data.rows.len() - 1;
                    }
                }
                Err(e) => {
                    context.show_error(format!("Failed to refresh table data: {e}"));
                }
            }
        }
    }

    // Navigation methods - exact copies from ApplicationState
    pub fn move_selection_up(&mut self) {
        log::debug!("TableDataViewer: move_selection_up called, current row: {}", self.selected_row);
        if self.selected_row > 0 {
            self.selected_row -= 1;
            log::debug!("TableDataViewer: moved up to row {}", self.selected_row);
            // Scroll up if selected row goes above visible area
            if self.selected_row < self.scroll_y {
                self.scroll_y = self.selected_row;
            }
        } else {
            log::debug!("TableDataViewer: already at top row");
        }
    }

    pub fn move_selection_down(&mut self, max_rows: usize, visible_rows: usize) {
        log::debug!("TableDataViewer: move_selection_down called, current row: {}, max_rows: {}", self.selected_row, max_rows);
        if self.selected_row + 1 < max_rows {
            self.selected_row += 1;
            log::debug!("TableDataViewer: moved down to row {}", self.selected_row);
            // Scroll down if selected row goes below visible area
            if self.selected_row >= self.scroll_y + visible_rows {
                self.scroll_y = self.selected_row - visible_rows + 1;
            }
        } else {
            log::debug!("TableDataViewer: already at bottom row");
        }
    }

    pub fn move_selection_left(&mut self, context: &AppContext) {
        log::debug!("TableDataViewer: move_selection_left called, current column: {:?}", self.selected_column);
        if let Some(ref current_col) = self.selected_column.clone() {
            if let Some(prev_col) = self.get_prev_visible_column(current_col, context) {
                log::debug!("TableDataViewer: moving left from {} to {}", current_col, prev_col);
                self.selected_column = Some(prev_col);
                // Ensure new selection is visible
                self.ensure_selected_column_visible();
            } else {
                log::debug!("TableDataViewer: no previous column found");
            }
        } else {
            // No selection, select first visible column
            log::debug!("TableDataViewer: no current selection, selecting first visible column");
            self.selected_column = self.get_first_visible_column(context);
        }
    }

    pub fn move_selection_right(&mut self, _max_cols: usize, visible_cols: usize, context: &AppContext) {
        if let Some(ref current_col) = self.selected_column.clone() {
            if let Some(next_col) = self.get_next_visible_column(current_col, context) {
                self.selected_column = Some(next_col);
                
                // Restore the original right-edge scrolling logic
                if let Some(selected_idx) = self.get_selected_column_index() {
                    // Scroll right if selected col goes right of visible area
                    if selected_idx >= self.scroll_x + visible_cols {
                        self.scroll_x = selected_idx - visible_cols + 1;
                    }
                }
            }
        } else {
            // No selection, select first visible column
            self.selected_column = self.get_first_visible_column(context);
        }
    }

    // Rendering methods - placeholders for exact copies from ui/mod.rs
    fn render_table_viewer(&self, _frame: &mut Frame, _area: Rect, _context: &AppContext, _db_manager: &DatabaseManager) {
        todo!("Copy exact implementation from ui/mod.rs::render_table_viewer (lines 1536-1633)")
    }

    fn render_table_widget(&self, frame: &mut Frame, area: Rect, data: &QueryResult, title: &str, context: &AppContext) {
        // Copy exact implementation from ui/mod.rs::render_table_widget (lines 738-913)
        let border_style = self.get_panel_border_style(context);
        
        // Create mapping from data column indices to original virtual column indices
        // Since hidden columns are filtered out in SQL, data.columns only contains visible columns
        let visible_columns = self.get_visible_columns();
        
        // Calculate visible rows and columns (exact copy from legacy)
        let available_height = area.height.saturating_sub(3) as usize; // Subtract borders and header
        let start_row = self.scroll_y;
        let end_row = (start_row + available_height).min(data.rows.len());
        
        // Calculate column constraints and visible columns (exact copy from legacy)
        let mut constraints: Vec<Constraint> = Vec::new();
        let mut visible_cols: Vec<usize> = Vec::new();
        let available_width = area.width.saturating_sub(2) as usize; // Subtract borders
        let start_col = self.scroll_x;
        let mut used_width = 0;
        
        // Calculate individual column widths based on content (exact copy from legacy)
        let min_col_width = 8;
        let mut column_widths = Vec::new();
        
        for (data_col_idx, col_name) in data.columns.iter().enumerate() {
            // Map data column index to original virtual column index
            let virtual_col_idx = visible_columns.get(data_col_idx).copied().unwrap_or(data_col_idx);
            
            // Calculate header width using the final header text (including sort indicators)
            let final_header_text = self.get_final_header_text(virtual_col_idx, col_name);
            let header_width = final_header_text.chars().count();
            let mut max_data_width = 0;
            
            // Calculate max data width for visible rows (exact copy from legacy)
            if self.is_column_expanded(virtual_col_idx) {
                // For expanded columns, check all visible rows for more accurate width
                for row in data.rows[start_row..end_row].iter() {
                    if let Some(cell) = row.get(data_col_idx) {
                        max_data_width = max_data_width.max(cell.chars().count());
                    }
                }
            } else {
                // For normal columns, sample first 10 rows for performance
                for row in data.rows.iter().take(10) {
                    if let Some(cell) = row.get(data_col_idx) {
                        max_data_width = max_data_width.max(cell.chars().count());
                    }
                }
            }
            
            let col_width = if self.is_column_expanded(virtual_col_idx) {
                // Expanded column: fit content up to max of 50 characters (exact copy from legacy)
                header_width.max(max_data_width).max(min_col_width).min(50)
            } else {
                // Normal column: limit to 25 characters (exact copy from legacy)
                header_width.max(max_data_width).max(min_col_width).min(25)
            };
            
            column_widths.push(col_width);
        }
        
        // Determine visible columns (exact copy from legacy lines 799-814)
        for i in start_col..data.columns.len() {
            let col_width = column_widths.get(i).unwrap_or(&min_col_width);
            if used_width + col_width <= available_width {
                constraints.push(Constraint::Length(*col_width as u16));
                visible_cols.push(i);
                used_width += col_width;
            } else {
                break;
            }
        }

        // Ensure we show at least one column (exact copy from legacy lines 811-814)
        if visible_cols.is_empty() && start_col < data.columns.len() {
            visible_cols.push(start_col);
            constraints.push(Constraint::Length(min_col_width as u16));
        }

        // Create header with styling for selected column and sort indicators (exact copy from legacy lines 817-839)
        let header_cells: Vec<Cell> = visible_cols.iter().map(|&data_col_idx| {
            let cell_content = &data.columns[data_col_idx];
            let col_width = column_widths.get(data_col_idx).unwrap_or(&min_col_width);
            
            // Map data column index to virtual column index
            let virtual_col_idx = visible_columns.get(data_col_idx).copied().unwrap_or(data_col_idx);
            
            // Get the complete header text with all indicators
            let header_with_sort = self.get_final_header_text(virtual_col_idx, cell_content);
            
            let text = if self.is_column_expanded(virtual_col_idx) {
                // For expanded columns, show full header without truncation
                header_with_sort
            } else {
                Self::truncate_text(&header_with_sort, *col_width)
            };
            
            if Some(virtual_col_idx) == self.get_selected_column_index() {
                Cell::from(text).style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            } else {
                Cell::from(text)
            }
        }).collect();

        // Create data rows with text wrapping support for expanded columns (exact copy from legacy lines 842-900)
        let mut rows: Vec<Row> = Vec::new();
        
        for (display_idx, row) in data.rows[start_row..end_row].iter().enumerate() {
            let actual_row_idx = start_row + display_idx;
            let is_selected_row = actual_row_idx == self.selected_row;
            
            // First, prepare wrapped content for all cells in this row
            let mut cell_lines: Vec<Vec<String>> = Vec::new();
            let mut max_lines = 1;
            
            for &data_col_idx in &visible_cols {
                let empty_string = String::new();
                let cell_content = row.get(data_col_idx).unwrap_or(&empty_string);
                let col_width = column_widths.get(data_col_idx).unwrap_or(&min_col_width);
                
                // Map data column index to virtual column index
                let virtual_col_idx = visible_columns.get(data_col_idx).copied().unwrap_or(data_col_idx);
                
                let lines = if self.is_column_expanded(virtual_col_idx) {
                    Self::wrap_text(cell_content, *col_width)
                } else {
                    vec![Self::truncate_text(cell_content, *col_width)]
                };
                
                max_lines = max_lines.max(lines.len());
                cell_lines.push(lines);
            }
            
            // Create multiple rows if any cell has wrapped content
            for line_idx in 0..max_lines {
                let cells: Vec<Cell> = visible_cols.iter().enumerate().map(|(visible_idx, &data_col_idx)| {
                    let line_text = cell_lines.get(visible_idx)
                        .and_then(|lines| lines.get(line_idx))
                        .unwrap_or(&String::new())
                        .clone();
                    
                    // Map data column index to virtual column index
                    let virtual_col_idx = visible_columns.get(data_col_idx).copied().unwrap_or(data_col_idx);
                    
                    // Check if this is the current cell (intersection of selected row and column)
                    let is_current_cell = is_selected_row && Some(virtual_col_idx) == self.get_selected_column_index();
                    
                    if is_current_cell {
                        // Highlight current cell with light gray background and inverted text for readability
                        Cell::from(line_text).style(Style::default().bg(Color::Gray).fg(Color::Black).add_modifier(Modifier::BOLD))
                    } else if is_selected_row {
                        // Bold selected row
                        Cell::from(line_text).style(Style::default().add_modifier(Modifier::BOLD))
                    } else if Some(virtual_col_idx) == self.get_selected_column_index() {
                        // Subtle highlight for selected column
                        Cell::from(line_text).style(Style::default().fg(Color::Gray))
                    } else {
                        Cell::from(line_text)
                    }
                }).collect();
                
                rows.push(Row::new(cells));
            }
        }

        // Create final Table widget (exact copy from legacy lines 902-912)
        let table = Table::new(rows, constraints)
            .header(Row::new(header_cells).height(1))
            .block(Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(border_style))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol(">>")
            .column_spacing(1);

        frame.render_widget(table, area);
    }

    fn render_inspect_view(&self, frame: &mut Frame, area: Rect, table_name: &str, context: &AppContext, _db_manager: &DatabaseManager) {
        // Use cached schema and statistics data (loaded once when entering inspect mode)
        let schema_data = self.inspect_schema_data.as_ref()
            .cloned()
            .unwrap_or_else(|| crate::db::query::QueryResult::new());
        let stats_data = self.inspect_stats_data.as_ref()
            .cloned()
            .unwrap_or_else(|| crate::db::query::QueryResult::new());

        // Split area into two sections: schema on top, statistics on bottom
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(50), // Schema section
                Constraint::Percentage(50), // Statistics section
            ])
            .split(area);

        // Render schema section
        self.render_schema_section(frame, chunks[0], table_name, &schema_data, context);

        // Render statistics section
        self.render_statistics_section(frame, chunks[1], table_name, &stats_data);
    }

    fn render_schema_section(&self, f: &mut Frame, area: Rect, table_name: &str, schema_data: &crate::db::query::QueryResult, context: &AppContext) {
        // Determine if this section is active and style accordingly
        let is_active = matches!(self.inspect_active_section, InspectSection::Schema);
        let border_style = if is_active {
            Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        // Create title with active indicator
        let title = if is_active {
            if self.is_modifying {
                format!("► Columns: {table_name} (MODIFY MODE - j/k to move, J/K for extremes, o to hide/show, Enter to confirm, Esc to cancel)")
            } else {
                format!("► Columns: {table_name} (Tab to switch, j/k to scroll, m to modify)")
            }
        } else {
            format!("Columns: {table_name}")
        };

        if schema_data.rows.is_empty() {
            // No schema data available
            let content = "No column information available for this table.";
            let paragraph = Paragraph::new(content)
                .block(
                    Block::default()
                        .title(title)
                        .borders(Borders::ALL)
                        .border_style(border_style),
                )
                .wrap(Wrap { trim: true })
                .style(Style::default().fg(Color::White));
            f.render_widget(paragraph, area);
            return;
        }

        // Build table rows from schema data, respecting virtual column order
        let virtual_order = self.get_virtual_column_order(context);
        let original_columns = &self.original_column_names;

        // Reorder schema data to match virtual column order
        let mut ordered_schema_rows = Vec::new();
        for virtual_col_name in &virtual_order {
            // Find the index of this column in the original schema
            if let Some(original_idx) = original_columns.iter().position(|name| name == virtual_col_name) {
                if original_idx < schema_data.rows.len() {
                    ordered_schema_rows.push(schema_data.rows[original_idx].clone());
                }
            }
        }

        // Create header in virtual order (add extra columns for enhanced display)
        let header = vec![
            Cell::from("Column Name"),
            Cell::from("Data Type"),
            Cell::from("Nullable"),
            Cell::from("Sort Order"),
            Cell::from("Sort Direction"),
            Cell::from("Hidden"),
        ];

        // Apply scrolling: skip rows based on scroll position
        let scroll_offset = self.inspect_schema_scroll_y;
        let rows: Vec<Row> = ordered_schema_rows.iter()
            .enumerate()
            .skip(scroll_offset)
            .map(|(row_idx, row)| {
                // Get the virtual column index
                // Note: row_idx already accounts for skip() since enumerate() happens before skip()
                let virtual_col_idx = row_idx;

                // Get the column name from virtual order
                let column_name = if virtual_col_idx < virtual_order.len() {
                    &virtual_order[virtual_col_idx]
                } else {
                    return Row::new(vec![Cell::from("ERROR")]);
                };

                // Check if this column is hidden by name
                let is_hidden = self.is_column_hidden_by_name(column_name, context);

                // Get sort information
                let sort_info = self.sort_columns.iter()
                    .position(|spec| spec.column_name == *column_name)
                    .map(|pos| (pos + 1, &self.sort_columns[pos].direction));

                let (sort_order, sort_direction) = match sort_info {
                    Some((order, direction)) => {
                        let dir_str = match direction {
                            SortDirection::Ascending => "ASC",
                            SortDirection::Descending => "DESC",
                        };
                        (order.to_string(), dir_str.to_string())
                    },
                    None => ("".to_string(), "".to_string()),
                };

                // Build enhanced row with additional columns
                let base_style = if is_hidden {
                    Style::default().fg(Color::DarkGray)
                } else {
                    Style::default()
                };

                let cells = vec![
                    Cell::from(row.first().unwrap_or(&"?".to_string()).clone()).style(base_style),  // Column Name
                    Cell::from(row.get(1).unwrap_or(&"?".to_string()).clone()).style(base_style),  // Data Type
                    Cell::from(row.get(2).unwrap_or(&"?".to_string()).clone()).style(base_style),  // Nullable
                    Cell::from(sort_order.clone()).style(base_style),                              // Sort Order
                    Cell::from(sort_direction.clone()).style(base_style),                          // Sort Direction
                    Cell::from(if is_hidden { "YES" } else { "NO" }).style(base_style),            // Hidden
                ];

                Row::new(cells)
            }).collect();

        // Use fixed column widths for the enhanced display (6 columns)
        let constraints: Vec<Constraint> = vec![
            Constraint::Length(15),  // Column Name
            Constraint::Length(12),  // Data Type
            Constraint::Length(8),   // Nullable
            Constraint::Length(10),  // Sort Order
            Constraint::Length(12),  // Sort Direction
            Constraint::Length(6),   // Hidden
        ];

        let table = Table::new(rows, constraints)
            .header(Row::new(header).style(Style::default().add_modifier(Modifier::BOLD)))
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(border_style),
            )
            .highlight_style(if is_active {
                Style::default().bg(Color::Gray).fg(Color::Black).add_modifier(Modifier::BOLD)
            } else {
                Style::default().bg(Color::DarkGray)
            })
            .style(Style::default().fg(Color::White));

        // Create table state for selection
        let mut table_state = TableState::default();
        if is_active {
            // Show selection only when this section is active
            let selected_index = self.inspect_selected_row.saturating_sub(scroll_offset);
            table_state.select(Some(selected_index));
        }

        f.render_stateful_widget(table, area, &mut table_state);
    }

    fn render_statistics_section(&self, f: &mut Frame, area: Rect, table_name: &str, stats_data: &crate::db::query::QueryResult) {
        // Determine if this section is active and style accordingly
        let is_active = matches!(self.inspect_active_section, InspectSection::Statistics);
        let border_style = if is_active {
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        // Create title with active indicator
        let title = if is_active {
            format!("► Statistics: {table_name} (Tab to switch, ↑↓ to scroll, Esc to exit)")
        } else {
            format!("Statistics: {table_name} (Press Esc to exit inspect mode)")
        };

        if stats_data.rows.is_empty() {
            // No statistics data available
            let content = "No statistics available for this table.";
            let paragraph = Paragraph::new(content)
                .block(
                    Block::default()
                        .title(title)
                        .borders(Borders::ALL)
                        .border_style(border_style),
                )
                .wrap(Wrap { trim: true })
                .style(Style::default().fg(Color::White));
            f.render_widget(paragraph, area);
            return;
        }

        // Build table rows from statistics data
        let header = stats_data.columns.iter().map(|col| Cell::from(col.as_str())).collect::<Vec<_>>();

        // Apply scrolling: skip rows based on scroll position
        let scroll_offset = self.inspect_stats_scroll_y;
        let rows: Vec<Row> = stats_data.rows.iter()
            .skip(scroll_offset)
            .map(|row| {
                let cells: Vec<Cell> = row.iter().map(|cell| Cell::from(cell.as_str())).collect();
                Row::new(cells)
            }).collect();

        // Calculate column widths based on content
        let mut column_widths = vec![0; stats_data.columns.len()];

        // Check header widths
        for (i, col) in stats_data.columns.iter().enumerate() {
            column_widths[i] = col.len().max(column_widths[i]);
        }

        // Check data widths
        for row in &stats_data.rows {
            for (i, cell) in row.iter().enumerate() {
                if i < column_widths.len() {
                    column_widths[i] = cell.len().max(column_widths[i]);
                }
            }
        }

        // Convert to constraints with minimum and maximum widths
        let constraints: Vec<Constraint> = column_widths.iter().map(|&width| {
            let min_width = 8; // Minimum column width
            let max_width = 20; // Slightly smaller max for statistics to fit more columns
            let adjusted_width = width.max(min_width).min(max_width);
            Constraint::Length(adjusted_width as u16)
        }).collect();

        let table = Table::new(rows, constraints)
            .header(Row::new(header).style(Style::default().add_modifier(Modifier::BOLD)))
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(border_style),
            )
            .highlight_style(Style::default().bg(Color::DarkGray))
            .style(Style::default().fg(Color::White));

        f.render_widget(table, area);
    }

    // Helper methods - exact copies from ui/mod.rs
    fn get_panel_border_style(&self, context: &AppContext) -> Style {
        // Copy exact logic from ui/mod.rs::get_panel_border_style (lines 1516-1534)
        // Adapted for state pattern - table viewer is active when this state is active
        if self.is_active {
            if context.is_panel_flashing() {
                // Flash effect: bright cyan (exact copy)
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                // Normal active: green (exact copy)
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            }
        } else {
            // Inactive: default (exact copy)
            Style::default()
        }
    }

    fn get_visible_columns(&self) -> Vec<usize> {
        // TODO: Copy exact implementation from ApplicationState::get_visible_columns
        // For now, return simple sequential indices based on table data
        if let Some(ref data) = self.table_data {
            (0..data.columns.len()).collect()
        } else {
            Vec::new()
        }
    }

    fn get_final_header_text(&self, _virtual_col_idx: usize, col_name: &str) -> String {
        // TODO: Copy exact implementation from ui/mod.rs with sort indicators
        // For now, return the column name (sort indicators will be added later)
        col_name.to_string()
    }

    // Modify mode methods
    pub fn start_modifying(&mut self) {
        todo!("Copy exact implementation from ApplicationState::start_modifying")
    }

    pub fn cancel_modifying(&mut self) {
        todo!("Copy exact implementation from ApplicationState::cancel_modifying")
    }

    // View creation methods
    pub fn start_view_name_input(&mut self) {
        todo!("Copy exact implementation from ApplicationState::start_view_name_input")
    }

    pub fn cancel_view_name_input(&mut self) {
        todo!("Copy exact implementation from ApplicationState::cancel_view_name_input")
    }

    // Extreme navigation methods (exact copies from ApplicationState)
    pub fn navigate_extreme_left(&mut self, context: &AppContext) {
        if !self.is_modifying {
            if self.inspect_mode {
                // Page left in inspect mode
                for _ in 0..10 {
                    self.inspect_scroll_up();
                }
            } else {
                // Go to leftmost column in table viewer
                self.scroll_x = 0;
                // Set selected column to first visible column
                if let Some(ref data) = self.table_data {
                    self.selected_column = data.columns.first().cloned();
                }
            }
        }
    }

    pub fn navigate_extreme_right(&mut self, context: &AppContext) {
        if !self.is_modifying {
            if self.inspect_mode {
                // Page right in inspect mode
                let (max_rows, visible_rows) = (20, 10); // Estimates - UI will provide better values
                for _ in 0..10 {
                    self.inspect_scroll_down(max_rows, visible_rows);
                }
            } else {
                // Go to rightmost column in table viewer
                self.selected_column = self.get_last_visible_column(context);
                // Estimate visible columns and scroll to show rightmost
                let total_cols = self.get_column_names(context).len();
                let estimated_visible_cols = 5;
                if total_cols > estimated_visible_cols {
                    self.scroll_x = total_cols - estimated_visible_cols;
                }
            }
        }
    }

    pub fn navigate_extreme_up(&mut self) {
        if !self.is_modifying {
            if self.inspect_mode {
                // Go to top in inspect mode
                if matches!(self.inspect_active_section, InspectSection::Schema) {
                    self.inspect_selected_row = 0;
                    self.inspect_schema_scroll_y = 0;
                } else {
                    self.inspect_stats_scroll_y = 0;
                }
            } else {
                // Go to first row in table viewer
                self.selected_row = 0;
                self.scroll_y = 0;
            }
        }
    }

    pub fn navigate_extreme_down(&mut self) {
        if !self.is_modifying {
            if self.inspect_mode {
                // Go to bottom in inspect mode
                if matches!(self.inspect_active_section, InspectSection::Schema) {
                    if let Some(ref data) = self.table_data {
                        let total_cols = data.columns.len();
                        if total_cols > 0 {
                            self.inspect_selected_row = total_cols - 1;
                            // Scroll to show bottom
                            let estimated_visible_rows = 10;
                            if total_cols > estimated_visible_rows {
                                self.inspect_schema_scroll_y = total_cols - estimated_visible_rows;
                            }
                        }
                    }
                } else {
                    // Go to bottom of statistics - estimate scroll
                    let estimated_max_rows = 20;
                    let estimated_visible_rows = 10;
                    if estimated_max_rows > estimated_visible_rows {
                        self.inspect_stats_scroll_y = estimated_max_rows - estimated_visible_rows;
                    }
                }
            } else if let Some(ref data) = self.table_data {
                // Go to last row in table viewer
                let total_rows = data.rows.len();
                if total_rows > 0 {
                    self.selected_row = total_rows - 1;
                    // Scroll to show bottom
                    let estimated_visible_rows = 10;
                    if total_rows > estimated_visible_rows {
                        self.scroll_y = total_rows - estimated_visible_rows;
                    }
                }
            }
        }
    }

    pub fn get_last_visible_column(&self, context: &AppContext) -> Option<String> {
        let column_names = self.get_virtual_column_order(context);
        for name in column_names.iter().rev() {
            if !self.is_column_hidden_by_name(name, context) {
                return Some(name.clone());
            }
        }
        None
    }

    // Column navigation helper methods (exact copies from ApplicationState)
    pub fn get_column_names(&self, context: &AppContext) -> Vec<String> {
        // Always return the virtual column order (which respects reordering)
        // This ensures all column operations work with the current display order
        self.get_virtual_column_order(context)
    }
    
    pub fn get_virtual_column_order(&self, context: &AppContext) -> Vec<String> {
        if let Some(table_name) = &context.selected_table {
            // Return custom order if exists, otherwise return original order
            self.column_order.get(table_name)
                .cloned()
                .unwrap_or_else(|| self.original_column_names.clone())
        } else {
            self.original_column_names.clone()
        }
    }

    pub fn is_column_hidden_by_name(&self, column_name: &str, context: &AppContext) -> bool {
        if let Some(table_name) = &context.selected_table {
            self.hidden_columns
                .get(table_name)
                .is_some_and(|hidden_set| hidden_set.contains(column_name))
        } else {
            false
        }
    }

    pub fn get_visible_column_names(&self, context: &AppContext) -> Vec<String> {
        let column_names = self.get_column_names(context);
        column_names.into_iter()
            .filter(|name| !self.is_column_hidden_by_name(name, context))
            .collect()
    }

    pub fn get_next_visible_column(&self, current_column: &str, context: &AppContext) -> Option<String> {
        let column_names = self.get_virtual_column_order(context);
        if let Some(current_idx) = column_names.iter().position(|name| name == current_column) {
            for idx in (current_idx + 1)..column_names.len() {
                if let Some(name) = column_names.get(idx) {
                    if !self.is_column_hidden_by_name(name, context) {
                        return Some(name.clone());
                    }
                }
            }
        }
        None
    }

    pub fn get_prev_visible_column(&self, current_column: &str, context: &AppContext) -> Option<String> {
        let column_names = self.get_virtual_column_order(context);
        if let Some(current_idx) = column_names.iter().position(|name| name == current_column) {
            for idx in (0..current_idx).rev() {
                if let Some(name) = column_names.get(idx) {
                    if !self.is_column_hidden_by_name(name, context) {
                        return Some(name.clone());
                    }
                }
            }
        }
        None
    }

    pub fn get_first_visible_column(&self, context: &AppContext) -> Option<String> {
        let column_names = self.get_virtual_column_order(context);
        column_names.into_iter().find(|name| !self.is_column_hidden_by_name(name, context))
    }

    fn ensure_selected_column_visible(&mut self) {
        // With name-based selection, we need to ensure the selected column index is visible
        if let Some(selected_idx) = self.get_selected_column_index() {
            if selected_idx < self.scroll_x {
                // Selected column is to the left of current view - scroll left to show it
                self.scroll_x = selected_idx;
            }
        }
        // Note: We can't easily calculate right-edge visibility here without knowing
        // the viewport width, but the existing logic should handle that
    }

    // Helper functions for text processing (exact copies from ui/mod.rs)
    fn truncate_text(text: &str, max_width: usize) -> String {
        if max_width <= 3 {
            return "...".to_string();
        }
        
        // Count characters, not bytes, for proper Unicode handling
        let char_count = text.chars().count();
        if char_count <= max_width {
            text.to_string()
        } else {
            // Take only the characters that fit, respecting Unicode boundaries
            let truncated: String = text.chars()
                .take(max_width.saturating_sub(3))
                .collect();
            format!("{truncated}...")
        }
    }

    /// Wrap text to fit within a specific width, returning wrapped lines
    /// This function properly handles Unicode character boundaries
    fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
        if max_width == 0 {
            return vec![];
        }
        
        let char_count = text.chars().count();
        if char_count <= max_width {
            return vec![text.to_string()];
        }
        
        let mut lines = Vec::new();
        let chars: Vec<char> = text.chars().collect();
        
        let mut start = 0;
        while start < chars.len() {
            let end = (start + max_width).min(chars.len());
            let line: String = chars[start..end].iter().collect();
            lines.push(line);
            start = end;
        }
        
        lines
    }
}

impl UIState for TableDataViewerState {
    fn render(
        &self,
        frame: &mut Frame,
        area: Rect,
        context: &AppContext,
        db_manager: &DatabaseManager,
    ) {
        // Copy exact logic from ui/mod.rs::render_table_viewer (lines 1536-1633)
        // Adapted to use context instead of ApplicationState
        
        // Check if we're in inspect mode and have a selected table
        if self.inspect_mode {
            if let Some(table) = context.selected_table.clone() {
                self.render_inspect_view(frame, area, &table, context, db_manager);
                return;
            }
        }

        // Cache area height for navigation calculations (exact copy from legacy)
        // Note: This will need to be made mutable when we implement the full logic
        // For now, we'll just use the area height for calculations
        let _area_height = area.height;
        
        let border_style = self.get_panel_border_style(context);

        let (content, title): (String, String) = if let Some(ref creation_step) = self.table_creation_step {
            // Copy exact table creation logic from legacy (lines 1549-1576)
            match creation_step {
                TableCreationStep::EnteringTableName(table_name) => {
                    let content = format!(
                        "Create New Table\n\nTable name: {}\n\nType the table name and press Enter to continue\nPress Esc to cancel",
                        if table_name.is_empty() { 
                            "_" 
                        } else { 
                            table_name 
                        }
                    );
                    (content, "Import Wizard - Table Name".to_string())
                }
                TableCreationStep::SelectingFile(table_name) => {
                    let content = format!(
                        "Create New Table: '{}'\n\nSelect a file to import data from:\n• CSV files (.csv)\n• JSON files (.json)\n• Parquet files (.parquet)\n\nPress Esc to cancel",
                        table_name
                    );
                    (content, "Import Wizard - File Selection".to_string())
                }
                TableCreationStep::ImportingData(table_name) => {
                    let content = format!(
                        "Create New Table: '{}'\n\nImporting data...\n\nPlease wait while the data is being imported.",
                        table_name
                    );
                    (content, "Import Wizard - Importing".to_string())
                }
            }
        } else if let (Some(db), Some(table)) = (&context.current_database, &context.selected_table) {
            if let Some(ref data) = self.table_data {
                // Copy exact table rendering logic from legacy (lines 1578-1602)
                // Build table title with sort status (exact copy)
                let sort_info = if !self.sort_columns.is_empty() {
                    let mut sort_parts = Vec::new();
                    for sort_spec in &self.sort_columns {
                        let direction = match sort_spec.direction {
                            SortDirection::Ascending => "ASC",
                            SortDirection::Descending => "DESC",
                        };
                        sort_parts.push(format!("{} {}", sort_spec.column_name, direction));
                    }
                    
                    if !sort_parts.is_empty() {
                        format!(" - Sorted by: {}", sort_parts.join(", "))
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                };
                
                let title = format!("Table: {} ({} rows){}", table, data.row_count, sort_info);
                self.render_table_widget(frame, area, data, &title, context);
                return; // Early return since we handled rendering directly
            } else {
                // Copy exact loading message from legacy (lines 1603-1608)
                let content = format!(
                    "Database: {db}\nTable: {table}\n\nLoading table data...\n\nPress Enter on table to load data or wait for auto-load"
                );
                (content, "Table Viewer".to_string())
            }
        } else {
            // Copy exact default message from legacy (lines 1609-1620)
            let debug_info = format!(
                "Database: {:?}\nTable: {:?}", 
                context.current_database, 
                context.selected_table
            );
            let content = format!(
                "Select a database and table to view data\n\nNavigation:\n• Use Tab/Shift+Tab to switch panels\n• Use ↑↓ to navigate lists\n• Press Enter to select items\n• Press i to import data\n• Press h for help\n• Press q or Esc to quit\n\nDebug:\n{}", 
                debug_info
            );
            (content, "Main Content [3]".to_string())
        };

        // Copy exact paragraph rendering from legacy (lines 1622-1632)
        let paragraph = Paragraph::new(content)
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(border_style),
            )
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(Color::White));

        frame.render_widget(paragraph, area);
    }

    fn handle_event(
        &mut self,
        event: KeyEvent,
        context: &mut AppContext,
        db_manager: &mut DatabaseManager,
    ) -> StateTransition {
        log::debug!("TableDataViewer: handle_event called with key: {:?}", event.code);
        log::debug!("TableDataViewer: is_active={}, has_table_data={}", self.is_active, context.table_data.is_some());

        // Copy exact key handling logic from ui/mod.rs for MainContent panel
        match event.code {
            // Vim navigation keys (exact copy from legacy lines 400-407)
            KeyCode::Char('k') | KeyCode::Up => {
                // Handle inspect mode navigation first (exact copy from legacy lines 521-531)
                if self.inspect_mode {
                    if matches!(self.inspect_active_section, InspectSection::Schema) {
                        // Move selection up in columns view
                        if self.inspect_selected_row > 0 {
                            self.inspect_selected_row -= 1;
                            if self.inspect_selected_row < self.inspect_schema_scroll_y {
                                self.inspect_schema_scroll_y = self.inspect_selected_row;
                            }
                        }
                    } else {
                        // Scroll up in statistics view
                        self.inspect_scroll_up();
                    }
                    return StateTransition::Stay;
                }

                // Normal table navigation
                if let Some(ref _data) = context.table_data {
                    self.move_selection_up();
                }
                StateTransition::Stay
            }
            KeyCode::Char('j') | KeyCode::Down => {
                // Handle inspect mode navigation first (exact copy from legacy lines 564-580)
                if self.inspect_mode {
                    if matches!(self.inspect_active_section, InspectSection::Schema) {
                        // Move selection down in columns view
                        let total_columns = self.get_virtual_column_order(context).len();
                        if self.inspect_selected_row + 1 < total_columns {
                            self.inspect_selected_row += 1;
                            // Calculate visible rows for scrolling
                            let visible_rows = 10; // Approximate
                            if self.inspect_selected_row >= self.inspect_schema_scroll_y + visible_rows {
                                self.inspect_schema_scroll_y = self.inspect_selected_row - visible_rows + 1;
                            }
                        }
                    } else {
                        // Scroll down in statistics view
                        let (_max_rows, visible_rows) = (50, 10); // Approximate bounds
                        self.inspect_scroll_down(_max_rows, visible_rows);
                    }
                    return StateTransition::Stay;
                }

                // Normal table navigation
                if let Some(ref data) = context.table_data {
                    let visible_rows = 10; // Approximate - could be calculated from area
                    self.move_selection_down(data.rows.len(), visible_rows);
                }
                StateTransition::Stay
            }
            KeyCode::Char('h') | KeyCode::Left => {
                // Handle inspect mode navigation - treat left as page up (exact copy from legacy lines 610-616)
                if self.inspect_mode {
                    for _ in 0..5 {
                        self.inspect_scroll_up();
                    }
                    return StateTransition::Stay;
                }

                // Normal table navigation
                if let Some(ref _data) = context.table_data {
                    self.move_selection_left(context);
                }
                StateTransition::Stay
            }
            KeyCode::Char('l') | KeyCode::Right => {
                // Handle inspect mode navigation - treat right as page down (exact copy from legacy lines 657-664)
                if self.inspect_mode {
                    let (max_rows, visible_rows) = (50, 10); // Approximate bounds
                    for _ in 0..5 {
                        self.inspect_scroll_down(max_rows, visible_rows);
                    }
                    return StateTransition::Stay;
                }

                // Normal table navigation
                if let Some(ref _data) = context.table_data {
                    let visible_cols = 5; // Approximate - could be calculated from area
                    self.move_selection_right(0, visible_cols, context);
                }
                StateTransition::Stay
            }
            // Extreme navigation keys (exact copy from legacy lines 404-407)
            KeyCode::Char('K') => {
                self.navigate_extreme_up();
                StateTransition::Stay
            }
            KeyCode::Char('J') => {
                self.navigate_extreme_down();
                StateTransition::Stay
            }
            KeyCode::Char('H') => {
                self.navigate_extreme_left(context);
                StateTransition::Stay
            }
            KeyCode::Char('L') => {
                self.navigate_extreme_right(context);
                StateTransition::Stay
            }
            // Enter key: toggle column expansion (exact copy from legacy ui/mod.rs lines 1344-1348)
            KeyCode::Enter => {
                if context.table_data.is_some() && !self.is_modifying {
                    self.toggle_column_expansion();
                }
                StateTransition::Stay
            }
            // Tab navigation (exact copy from legacy UI behavior)
            KeyCode::Tab => {
                if self.inspect_mode {
                    // In inspect mode: cycle between schema and statistics sections
                    self.inspect_cycle_section();
                    StateTransition::Stay
                } else {
                    // Go back to table list (matching legacy next_panel() behavior from MainContent) 
                    StateTransition::To(StateKey::TableSelect)
                }
            }
            KeyCode::BackTab => {
                // Go back to table list (matching legacy prev_panel() behavior)
                StateTransition::To(StateKey::TableSelect)
            }
            // Inspect mode toggle (exact copy from legacy ui/mod.rs lines 411-422)
            KeyCode::Char('i') => {
                if context.table_data.is_some() {
                    self.enter_inspect_mode();
                    // Load schema and statistics data once when entering inspect mode
                    if let Some(table_name) = &context.selected_table {
                        self.inspect_schema_data = db_manager.get_table_schema(table_name).ok();
                        // SUMMARIZE can fail on some tables (e.g., STDDEV_SAMP out of range)
                        // Log the error but continue - we'll show schema even if stats fail
                        match db_manager.get_table_statistics(table_name) {
                            Ok(stats) => self.inspect_stats_data = Some(stats),
                            Err(e) => {
                                log::warn!("Failed to get statistics for table {}: {}", table_name, e);
                                context.show_error(format!("Statistics unavailable: {e}"));
                                self.inspect_stats_data = None;
                            }
                        }
                    }
                }
                StateTransition::Stay
            }
            // Esc key (exact copy from legacy ui/mod.rs lines 384-394)
            KeyCode::Esc => {
                if self.is_modifying {
                    // Cancel modifying mode
                    self.cancel_modifying();
                    // TODO: fetch_table_data_preserve_column when modifying is transitioned
                } else if self.inspect_mode {
                    // Exit inspect mode
                    self.exit_inspect_mode();
                }
                StateTransition::Stay
            }
            // Sorting keys (exact copy from legacy ui/mod.rs lines 448-467)
            // Legacy only allows sorting when active_panel == MainContent (NOT in inspect mode)
            KeyCode::Char('a') => {
                // Toggle column in sort chain as ascending (only if table data exists and NOT in inspect mode)
                if self.table_data.is_some() && !self.inspect_mode {
                    self.toggle_in_sort_chain(true);
                    self.refresh_table_data(context, db_manager, true); // with limit
                }
                StateTransition::Stay
            }
            KeyCode::Char('A') => {
                // Toggle column in sort chain as descending (only if table data exists and NOT in inspect mode)
                if self.table_data.is_some() && !self.inspect_mode {
                    self.toggle_in_sort_chain(false);
                    self.refresh_table_data(context, db_manager, true); // with limit
                }
                StateTransition::Stay
            }
            KeyCode::Char('c') => {
                // Clear all sorting (only if table data exists and NOT in inspect mode)
                if self.table_data.is_some() && !self.inspect_mode {
                    self.clear_sort();
                    self.refresh_table_data(context, db_manager, true); // with limit
                }
                StateTransition::Stay
            }
            // Filtering keys (exact copy from legacy ui/mod.rs lines 469-484)
            // Legacy only allows filtering when active_panel == MainContent (NOT in inspect mode)
            KeyCode::Char('f') => {
                // Start column filter mode (only if table data exists and NOT in inspect mode)
                if self.table_data.is_some() && !self.inspect_mode {
                    if let Some(ref column_name) = self.selected_column {
                        // Push the filter input state with the column name
                        return StateTransition::PushState(StateKey::ColumnFilterInput(column_name.clone()));
                    }
                }
                StateTransition::Stay
            }
            KeyCode::Char('F') => {
                // Clear filter on selected column (only if table data exists and NOT in inspect mode)
                if self.table_data.is_some() && !self.inspect_mode {
                    if let Some(column_name) = self.selected_column.clone() {
                        self.clear_column_filter(&column_name);
                        context.column_filters.remove(&column_name); // Also remove from context
                        self.refresh_table_data(context, db_manager, false); // Legacy: no limit after filter clear (line 482)
                    }
                }
                StateTransition::Stay
            }
            KeyCode::Char('q') => StateTransition::Exit,
            _ => StateTransition::Stay,
        }
    }

    fn name(&self) -> &'static str {
        "TableDataViewerState"
    }

    fn on_enter(&mut self, context: &mut AppContext, db_manager: &mut DatabaseManager) {
        log::debug!("TableDataViewer: on_enter called");
        self.is_active = true;

        // Sync table data from context first
        self.sync(context, db_manager);

        // Detect if table changed by comparing current table with last viewed table
        let table_changed = self.last_viewed_table.as_ref() != context.selected_table.as_ref();

        if table_changed {
            log::debug!("TableDataViewer: table changed from {:?} to {:?}, resetting state", self.last_viewed_table, context.selected_table);
            // Reset all state when table changes (copy exact logic from ApplicationState::select_table lines 273-282)
            self.scroll_x = 0;
            self.scroll_y = 0;
            self.selected_row = 0;
            self.selected_column = None;
            self.expanded_columns.clear();
            self.sort_columns.clear();
            self.original_column_names.clear();

            // Update last viewed table
            self.last_viewed_table = context.selected_table.clone();

            // Initialize for the new table
            if let Some(ref table_data) = &self.table_data {
                log::debug!("TableDataViewer: initializing with {} columns, {} rows", table_data.columns.len(), table_data.rows.len());
                self.original_column_names = table_data.columns.clone();

                if !table_data.columns.is_empty() {
                    self.selected_column = Some(table_data.columns[0].clone());
                    log::debug!("TableDataViewer: initialized selected_column to: {:?}", self.selected_column);
                }

                log::debug!("TableDataViewer: initialized selection - row={}, col={:?}", self.selected_row, self.selected_column);
            }
        } else {
            log::debug!("TableDataViewer: re-entering same table, preserving navigation state");
            // Same table - preserve navigation state (scroll positions, selected row/col, etc.)
        }
    }

    fn on_exit(&mut self, _context: &mut AppContext, _db_manager: &mut DatabaseManager) {
        self.is_active = false;
        // TODO: Copy any cleanup logic from legacy UI
        // For now, just set inactive state - no additional cleanup needed for static rendering
    }

    fn sync(&mut self, context: &AppContext, _db_manager: &DatabaseManager) {
        // Sync state with context - copy table data, selection state, etc.
        // This will be crucial for maintaining consistency with legacy UI during transition
        if let Some(ref table_data) = context.table_data {
            self.table_data = Some(table_data.clone()); // hey claude! this is really heavy, do we
                                                        // need to clone here?
        } else {
            self.table_data = None;
        }
        
        if let Some(ref _selected_table) = context.selected_table {
            // TODO: Sync table-specific state like column orders, filters, etc.
            // For now, just ensure we have table data synced - detailed state sync will be added later
        }
    }
}
