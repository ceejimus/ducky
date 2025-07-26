use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};

use crate::db::query::QueryResult;
use crate::state::{StateContext, StateTransition, UIState};

/// Viewport state that tracks exactly what columns are rendered and their positions
#[derive(Debug, Clone)]
struct ViewportState {
    /// Available width for rendering columns
    available_width: usize,
    /// Width of each column in the table
    column_widths: Vec<usize>,
    /// Indices of currently visible columns
    visible_column_indices: Vec<usize>,
    /// Index of the leftmost visible column (scroll_x equivalent)
    leftmost_column_index: usize,
    /// Index of the rightmost visible column
    rightmost_column_index: usize,
    /// Total width consumed by visible columns
    used_width: usize,
}

impl ViewportState {
    fn new() -> Self {
        Self {
            available_width: 0,
            column_widths: Vec::new(),
            visible_column_indices: Vec::new(),
            leftmost_column_index: 0,
            rightmost_column_index: 0,
            used_width: 0,
        }
    }

    /// Check if a column index is currently visible
    fn is_column_visible(&self, column_index: usize) -> bool {
        self.visible_column_indices.contains(&column_index)
    }

    /// Get the number of columns currently rendered
    fn rendered_column_count(&self) -> usize {
        self.visible_column_indices.len()
    }
}

/// TableDataViewer manages the display and interaction with table data
pub struct TableDataViewer {
    // Scrolling and navigation state
    scroll_x: usize,
    scroll_y: usize,
    selected_row: usize,
    selected_column_index: usize,
    last_visible_rows: usize, // Cache the last calculated visible rows

    // Column expansion state - support multiple expanded columns
    expanded_columns: std::collections::HashSet<usize>,

    // Cached table data
    table_data: Option<QueryResult>,

    // Total row count for the table (for navigation limits)
    total_row_count: Option<usize>,

    // Deterministic viewport state
    viewport: ViewportState,
}

impl TableDataViewer {
    pub fn new() -> Self {
        Self {
            scroll_x: 0,
            scroll_y: 0,
            selected_row: 0,
            selected_column_index: 0,
            last_visible_rows: 20, // Default fallback
            expanded_columns: std::collections::HashSet::new(),
            table_data: None,
            total_row_count: None,
            viewport: ViewportState::new(),
        }
    }

    /// Toggle column expansion for the currently selected column
    fn toggle_column_expansion(&mut self) {
        if self.expanded_columns.contains(&self.selected_column_index) {
            self.expanded_columns.remove(&self.selected_column_index);
        } else {
            self.expanded_columns.insert(self.selected_column_index);
        }
    }

    /// Check if a column is expanded
    fn is_column_expanded(&self, column_index: usize) -> bool {
        self.expanded_columns.contains(&column_index)
    }

    /// Update viewport state deterministically
    fn update_viewport(&mut self, available_width: usize) {
        if let Some(ref data) = self.table_data {
            self.viewport.available_width = available_width;

            // Calculate column widths
            let min_col_width = 8;
            let mut column_widths = Vec::new();

            for (col_idx, col_name) in data.columns.iter().enumerate() {
                let header_width = col_name.chars().count();
                let mut max_data_width = 0;

                // Sample first 10 rows for performance
                for row in data.rows.iter().take(10) {
                    if let Some(cell) = row.get(col_idx) {
                        max_data_width = max_data_width.max(cell.chars().count());
                    }
                }

                let col_width = if self.is_column_expanded(col_idx) {
                    header_width.max(max_data_width).max(min_col_width).min(50)
                } else {
                    header_width.max(max_data_width).max(min_col_width).min(25)
                };

                column_widths.push(col_width);
            }

            self.viewport.column_widths = column_widths;

            // Calculate visible columns starting from scroll_x
            let mut visible_indices = Vec::new();
            let mut used_width = 0;

            for i in self.scroll_x..data.columns.len() {
                let col_width = self.viewport.column_widths.get(i).unwrap_or(&min_col_width);
                if used_width + col_width <= available_width {
                    visible_indices.push(i);
                    used_width += col_width;
                } else {
                    break;
                }
            }

            // Ensure at least one column is visible
            if visible_indices.is_empty() && self.scroll_x < data.columns.len() {
                visible_indices.push(self.scroll_x);
                used_width = *self
                    .viewport
                    .column_widths
                    .get(self.scroll_x)
                    .unwrap_or(&min_col_width);
            }

            // Update viewport state
            self.viewport.visible_column_indices = visible_indices.clone();
            self.viewport.used_width = used_width;
            self.viewport.leftmost_column_index = self.scroll_x;
            self.viewport.rightmost_column_index =
                visible_indices.last().copied().unwrap_or(self.scroll_x);
        }
    }

    /// Calculate how many rows can fit in the available area
    fn calculate_visible_rows(&self) -> usize {
        // Use cached value, will be updated during render
        self.last_visible_rows
    }

    /// Update the visible rows calculation based on area height
    fn update_visible_rows(&mut self, area_height: u16) {
        // Account for borders (2) and header (1)
        let available_height = area_height.saturating_sub(3) as usize;
        // Each data row takes 1 line
        self.last_visible_rows = available_height.max(1); // Ensure at least 1 row
    }

    /// Navigate to ensure a column is visible, adjusting scroll position deterministically
    fn ensure_column_visible(&mut self, column_index: usize) {
        if let Some(ref data) = self.table_data {
            if column_index >= data.columns.len() {
                return;
            }

            // If column is already visible, no need to scroll
            if self.viewport.is_column_visible(column_index) {
                return;
            }

            // If column is to the left of current view, scroll left to it
            if column_index < self.viewport.leftmost_column_index {
                self.scroll_x = column_index;
                self.update_viewport(self.viewport.available_width);
                return;
            }

            // If column is to the right, scroll right incrementally until it's visible
            if column_index > self.viewport.rightmost_column_index {
                // Find minimum scroll_x that makes the column visible
                for start_idx in (self.scroll_x + 1)..=column_index {
                    let mut test_visible = Vec::new();
                    let mut test_used_width = 0;
                    let min_col_width = 8;

                    for i in start_idx..data.columns.len() {
                        let col_width =
                            self.viewport.column_widths.get(i).unwrap_or(&min_col_width);
                        if test_used_width + col_width <= self.viewport.available_width {
                            test_visible.push(i);
                            test_used_width += col_width;
                        } else {
                            break;
                        }
                    }

                    if test_visible.contains(&column_index) {
                        self.scroll_x = start_idx;
                        self.update_viewport(self.viewport.available_width);
                        break;
                    }
                }
            }
        }
    }

    /// Wrap text to multiple lines for expanded columns
    fn wrap_text(&self, text: &str, max_width: usize) -> Vec<String> {
        let mut lines = Vec::new();
        let mut current_line = String::new();

        for word in text.split_whitespace() {
            if current_line.is_empty() {
                current_line = word.to_string();
            } else if current_line.chars().count() + 1 + word.chars().count() <= max_width {
                current_line.push(' ');
                current_line.push_str(word);
            } else {
                lines.push(current_line);
                current_line = word.to_string();
            }
        }

        if !current_line.is_empty() {
            lines.push(current_line);
        }

        if lines.is_empty() {
            lines.push(String::new());
        }

        lines
    }

    /// Public method to refresh table data - called when column order/visibility changes
    pub fn refresh_table_data(&mut self, context: &mut StateContext) {
        self.fetch_table_data_preserve_selection(context);
    }

    /// Handle column reorder mode completion - adjust selection if current column is hidden
    pub fn handle_column_reorder_completion(&mut self, context: &mut StateContext) {
        // Check if currently selected column is now hidden
        if let Some(ref current_col) = context.global_state.current_selected_column.clone() {
            if self.is_column_hidden_by_name(current_col, context) {
                tracing::debug!(
                    "Column {} is now hidden, adjusting selection",
                    current_col
                );

                // Store current column index for reference
                let current_col_index = self.get_selected_column_index(context);

                // Try to move left first (preferred behavior)
                if let Some(prev_col) =
                    self.get_prev_visible_column_with_context(current_col, context)
                {
                    context.global_state.current_selected_column = Some(prev_col);
                    tracing::debug!("Moved selection to previous visible column");
                } else if let Some(next_col) =
                    self.get_next_visible_column_with_context(current_col, context)
                {
                    // No previous column, try next column
                    context.global_state.current_selected_column = Some(next_col);
                    tracing::debug!("Moved selection to next visible column");
                } else {
                    // No visible columns at all, select first visible column if any exist
                    context.global_state.current_selected_column =
                        self.get_first_visible_column(context);
                    tracing::debug!("No adjacent visible columns, selected first visible column");
                }

                // Don't adjust viewport - keep current scroll position
                // Only ensure column is visible if it's way off screen
                if let Some(new_column_index) = self.get_selected_column_index(context) {
                    // Only adjust viewport if the new column is completely outside the current view
                    if !self.viewport.is_column_visible(new_column_index) {
                        // Check if we need minimal adjustment
                        if let Some(current_idx) = current_col_index {
                            // If the hidden column was visible, try to keep viewport similar
                            if self.viewport.is_column_visible(current_idx) {
                                // Hidden column was in view, minimal adjustment needed
                                self.ensure_column_visible(new_column_index);
                            }
                        }
                    }
                }
            } else {
                tracing::debug!("Current column is still visible, no adjustment needed");
            }
        } else {
            // No current selection, select first visible column but don't change viewport
            context.global_state.current_selected_column = self.get_first_visible_column(context);
            tracing::debug!("No current selection, selected first visible column");
        }
    }

    /// Fetch table data while preserving current selection
    fn fetch_table_data_preserve_selection(&mut self, context: &mut StateContext) {
        self.fetch_table_data_internal(context, false);
    }

    /// Fetch total row count for the current table
    fn fetch_total_row_count(&mut self, context: &StateContext) {
        if let Some(table_name) = &context.global_state.selected_table {
            if let Some(current_db) = context.database_manager.get_current_database() {
                if let Some(connection) = context.database_manager.get_connection(current_db) {
                    // Build count query with same filtering as main query
                    let mut count_sql = format!("SELECT COUNT(*) FROM {}", table_name);

                    // Add WHERE clause for filters (same as main query)
                    if !context.global_state.column_filters.is_empty() {
                        let conditions: Vec<String> = context
                            .global_state
                            .column_filters
                            .iter()
                            .map(|(column, filter_text)| {
                                // Use the filter text directly as SQL (main branch approach)
                                format!("{} {}", column, filter_text)
                            })
                            .collect();
                        count_sql.push_str(&format!(" WHERE {}", conditions.join(" AND ")));
                    }

                    if let Ok(mut stmt) = connection.prepare(&count_sql) {
                        if let Ok(mut rows) = stmt.query([]) {
                            if let Some(row) = rows.next().unwrap_or(None) {
                                if let Ok(count) = row.get::<_, i64>(0) {
                                    self.total_row_count = Some(count as usize);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Fetch table data for the current table (resets selection)
    fn fetch_table_data(&mut self, context: &mut StateContext) {
        self.fetch_table_data_internal(context, true);
    }

    /// Internal method to fetch table data with control over selection reset
    fn fetch_table_data_internal(&mut self, context: &mut StateContext, reset_selection: bool) {
        if let Some(table_name) = &context.global_state.selected_table {
            if let Some(current_db) = context.database_manager.get_current_database() {
                if let Some(connection) = context.database_manager.get_connection(current_db) {
                    // Fetch total row count first
                    self.fetch_total_row_count(context);

                    // Build SQL with filtering and sorting
                    let sql = self.build_table_query(table_name, context);
                    match self.execute_query_direct(connection, &sql) {
                        Ok(data) => {
                            self.table_data = Some(data);
                            // Only reset navigation if explicitly requested (e.g., fresh table load)
                            if reset_selection {
                                self.scroll_x = 0;
                                self.scroll_y = 0;
                                self.selected_row = 0;
                                context.global_state.current_selected_column =
                                    self.get_first_visible_column(context);
                                self.selected_column_index = 0;
                            }
                            tracing::info!("Loaded data for table: {}", table_name);
                        }
                        Err(e) => {
                            tracing::error!("Failed to load table data: {e}");
                            self.table_data = None;
                        }
                    }
                }
            }
        }
    }

    /// Build SQL query with filtering and sorting
    fn build_table_query(&self, table_name: &str, context: &StateContext) -> String {
        // Check if we have custom column order for this table
        let select_clause = if let Some(ordered_columns) =
            context.global_state.column_order.get(table_name)
        {
            // Filter out hidden columns
            let visible_columns: Vec<String> =
                if let Some(hidden_columns) = context.global_state.hidden_columns.get(table_name) {
                    ordered_columns
                        .iter()
                        .filter(|col| !hidden_columns.contains(*col))
                        .cloned()
                        .collect()
                } else {
                    ordered_columns.clone()
                };

            if visible_columns.is_empty() {
                "*".to_string() // Fallback if all columns are hidden
            } else {
                visible_columns.join(", ")
            }
        } else {
            "*".to_string() // Default: select all columns
        };

        let mut sql = format!("SELECT {} FROM {}", select_clause, table_name);

        // Add WHERE clause for filters
        if !context.global_state.column_filters.is_empty() {
            let conditions: Vec<String> = context
                .global_state
                .column_filters
                .iter()
                .map(|(column, filter_text)| {
                    // Use the filter text directly as SQL (main branch approach)
                    format!("{} {}", column, filter_text)
                })
                .collect();
            sql.push_str(&format!(" WHERE {}", conditions.join(" AND ")));
        }

        // Add ORDER BY clause for sorting
        if !context.global_state.sort_columns.is_empty() {
            let order_clauses: Vec<String> = context
                .global_state
                .sort_columns
                .iter()
                .map(|spec| {
                    let direction = match spec.direction {
                        crate::state::SortDirection::Ascending => "ASC",
                        crate::state::SortDirection::Descending => "DESC",
                    };
                    format!("{} {}", spec.column_name, direction)
                })
                .collect();
            sql.push_str(&format!(" ORDER BY {}", order_clauses.join(", ")));
        }

        // Add LIMIT and OFFSET for pagination
        let visible_rows = self.calculate_visible_rows();
        let offset = self.scroll_y;
        sql.push_str(&format!(" LIMIT {} OFFSET {}", visible_rows, offset));

        sql
    }

    /// Execute a query directly against the connection
    fn execute_query_direct(
        &self,
        connection: &duckdb::Connection,
        sql: &str,
    ) -> Result<QueryResult> {
        let start_time = std::time::Instant::now();

        // Prepare statement and execute query (following original approach)
        let mut stmt = connection.prepare(sql)?;
        let mut rows = stmt.query([])?;

        // Get column count from the rows result (not from stmt directly)
        let column_count = rows.as_ref().unwrap().column_count();

        // Verify we have columns (statement has a result set)
        if column_count == 0 {
            return Err(anyhow::anyhow!("Query returned no columns"));
        }

        // Get column names from the rows result
        let mut columns = Vec::new();
        for i in 0..column_count {
            let column_name = rows
                .as_ref()
                .unwrap()
                .column_name(i)
                .unwrap_or(&format!("column_{i}"))
                .to_string();
            columns.push(column_name);
        }

        // Collect all rows
        let mut result_rows = Vec::new();
        while let Some(row) = rows.next()? {
            let mut row_data = Vec::new();
            for i in 0..column_count {
                let value = self.format_column_value(&row, i)?;
                row_data.push(value);
            }
            result_rows.push(row_data);
        }

        let execution_time = start_time.elapsed();

        Ok(QueryResult {
            columns,
            row_count: result_rows.len(),
            rows: result_rows,
            execution_time_ms: execution_time.as_millis() as u64,
        })
    }

    /// Format a column value from a DuckDB row
    fn format_column_value(
        &self,
        row: &duckdb::Row,
        index: usize,
    ) -> Result<String, duckdb::Error> {
        // Try f64 first to handle NaN values safely, following main branch approach
        match row.get::<_, f64>(index) {
            Ok(v) => {
                if v.is_nan() {
                    Ok("NaN".to_string())
                } else if v.is_infinite() {
                    Ok(if v.is_sign_positive() {
                        "Infinity"
                    } else {
                        "-Infinity"
                    }
                    .to_string())
                } else {
                    Ok(v.to_string())
                }
            }
            Err(_) => {
                // Try string next
                match row.get::<_, String>(index) {
                    Ok(v) => Ok(v),
                    Err(_) => {
                        // Try integer
                        match row.get::<_, i64>(index) {
                            Ok(v) => Ok(v.to_string()),
                            Err(_) => {
                                // Try boolean
                                match row.get::<_, bool>(index) {
                                    Ok(v) => Ok(v.to_string()),
                                    Err(_) => {
                                        // Try to get as raw value or default to NULL
                                        match row.get_ref(index) {
                                            Ok(value_ref) => Ok(format!("{value_ref:?}")),
                                            Err(_) => Ok("NULL".to_string()),
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

impl UIState for TableDataViewer {
    fn render(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        is_active: bool,
        context: &mut StateContext,
    ) -> Result<()> {
        let border_style = if is_active {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };

        let title = if let Some(table_name) = &context.global_state.selected_table {
            format!("Data View: {}", table_name)
        } else {
            "Data View".to_string()
        };

        // If we have table data, render it as a table
        if self.table_data.is_some() {
            // Update visible rows calculation
            self.update_visible_rows(area.height);

            // Update viewport state deterministically
            let available_width = area.width.saturating_sub(2) as usize; // Subtract borders
            self.update_viewport(available_width);

            // Use viewport state for rendering
            let visible_cols = self.viewport.visible_column_indices.clone();
            let constraints: Vec<Constraint> = visible_cols
                .iter()
                .map(|&col_idx| {
                    let width = self.viewport.column_widths.get(col_idx).unwrap_or(&8);
                    Constraint::Length(*width as u16)
                })
                .collect();

            let data = self.table_data.as_ref().unwrap();

            // Create header with highlighting for selected column - only for visible columns
            let header_cells: Vec<Cell> = visible_cols
                .iter()
                .map(|&col_idx| {
                    let empty_string = String::new();
                    let col_name = data.columns.get(col_idx).unwrap_or(&empty_string);
                    let col_width = self.viewport.column_widths.get(col_idx).unwrap_or(&8);

                    // Build header text with sort and filter indicators
                    let mut header_text = col_name.clone();

                    // Add sort indicator if this column is in sort chain
                    if let Some((direction, order)) =
                        context.global_state.is_column_sorted(col_name)
                    {
                        let sort_symbol = match direction {
                            crate::state::SortDirection::Ascending => "↑",
                            crate::state::SortDirection::Descending => "↓",
                        };
                        header_text = format!("{}{}{}", header_text, sort_symbol, order);
                    }

                    // Add filter indicator if this column is filtered
                    if context.global_state.column_filters.contains_key(col_name) {
                        header_text = format!("{}🔍", header_text);
                    }

                    // Truncate if needed (accounting for indicators)
                    let truncated_header = if header_text.chars().count() > *col_width {
                        format!(
                            "{}...",
                            header_text
                                .chars()
                                .take(col_width.saturating_sub(3))
                                .collect::<String>()
                        )
                    } else {
                        header_text
                    };

                    let cell = Cell::from(truncated_header)
                        .style(Style::default().add_modifier(Modifier::BOLD));
                    if Some(col_name.clone()) == context.global_state.current_selected_column {
                        cell.style(
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD),
                        )
                    } else {
                        cell
                    }
                })
                .collect();

            let header = Row::new(header_cells)
                .style(Style::default().fg(Color::Yellow))
                .height(1);

            // Create data rows with cell highlighting - only for visible columns
            let rows: Vec<Row> = data
                .rows
                .iter()
                .take(self.last_visible_rows) // Use calculated visible rows
                .enumerate()
                .map(|(i, row)| {
                    let actual_row_idx = i + self.scroll_y;
                    let is_selected_row = actual_row_idx == self.selected_row;

                    let cells: Vec<Cell> = visible_cols
                        .iter()
                        .map(|&col_idx| {
                            let empty_string = String::new();
                            let cell_content = row.get(col_idx).unwrap_or(&empty_string);
                            let column_name =
                                data.columns.get(col_idx).cloned().unwrap_or_default();
                            let is_selected_column =
                                Some(column_name) == context.global_state.current_selected_column;
                            let is_current_cell = is_selected_row && is_selected_column;

                            // Truncate content to fit column width
                            let col_width = self.viewport.column_widths.get(col_idx).unwrap_or(&8);
                            let truncated_content = if cell_content.chars().count() > *col_width {
                                format!(
                                    "{}...",
                                    cell_content
                                        .chars()
                                        .take(col_width.saturating_sub(3))
                                        .collect::<String>()
                                )
                            } else {
                                cell_content.clone()
                            };

                            if is_current_cell {
                                // Highlight current cell (intersection of selected row and column)
                                Cell::from(truncated_content).style(
                                    Style::default()
                                        .bg(Color::Gray)
                                        .fg(Color::Black)
                                        .add_modifier(Modifier::BOLD),
                                )
                            } else if is_selected_row {
                                // Bold selected row
                                Cell::from(truncated_content)
                                    .style(Style::default().add_modifier(Modifier::BOLD))
                            } else if is_selected_column {
                                // Subtle highlight for selected column
                                Cell::from(truncated_content)
                                    .style(Style::default().fg(Color::Gray))
                            } else {
                                Cell::from(truncated_content)
                            }
                        })
                        .collect();

                    Row::new(cells).height(1)
                })
                .collect();

            let table = Table::new(rows, constraints)
                .header(header)
                .block(
                    Block::default()
                        .title(title.as_str())
                        .borders(Borders::ALL)
                        .border_style(border_style),
                )
                .style(Style::default().fg(Color::White));

            frame.render_widget(table, area);
        } else {
            // No data available - show placeholder
            let placeholder = Paragraph::new("No data loaded - press 'r' to refresh")
                .block(
                    Block::default()
                        .title(title.as_str())
                        .borders(Borders::ALL)
                        .border_style(border_style),
                )
                .style(Style::default().fg(Color::White));

            frame.render_widget(placeholder, area);
        }

        Ok(())
    }

    fn handle_event(&mut self, event: KeyEvent, context: &mut StateContext) -> StateTransition {
        match event.code {
            KeyCode::Left | KeyCode::Char('h') => {
                // Move selection left (to previous column)
                if let Some(ref current_col) = context.global_state.current_selected_column.clone()
                {
                    if let Some(prev_col) =
                        self.get_prev_visible_column_with_context(current_col, context)
                    {
                        context.global_state.current_selected_column = Some(prev_col);
                        if let Some(column_index) = self.get_selected_column_index(context) {
                            self.ensure_column_visible(column_index);
                        }
                    }
                } else {
                    // No selection, select first visible column
                    context.global_state.current_selected_column =
                        self.get_first_visible_column(context);
                }
                StateTransition::Stay
            }
            KeyCode::Right | KeyCode::Char('l') => {
                // Move selection right (to next column)
                if let Some(ref current_col) = context.global_state.current_selected_column.clone()
                {
                    if let Some(next_col) =
                        self.get_next_visible_column_with_context(current_col, context)
                    {
                        context.global_state.current_selected_column = Some(next_col);
                        if let Some(column_index) = self.get_selected_column_index(context) {
                            self.ensure_column_visible(column_index);
                        }
                    }
                } else {
                    // No selection, select first visible column
                    context.global_state.current_selected_column =
                        self.get_first_visible_column(context);
                }
                StateTransition::Stay
            }
            KeyCode::Up | KeyCode::Char('k') => {
                // Move selection up
                if self.selected_row > 0 {
                    self.selected_row -= 1;
                    // Check if we need to scroll the view up
                    if self.selected_row < self.scroll_y {
                        self.scroll_y = self.selected_row;
                        self.fetch_table_data_preserve_selection(context); // Reload data with new offset
                    }
                }
                StateTransition::Stay
            }
            KeyCode::Down | KeyCode::Char('j') => {
                // Move selection down - check against total row count, not current data
                let max_row = if let Some(total) = self.total_row_count {
                    total.saturating_sub(1)
                } else {
                    // Fallback to current data size if total not available
                    self.table_data
                        .as_ref()
                        .map(|d| d.rows.len().saturating_sub(1))
                        .unwrap_or(0)
                };

                if self.selected_row < max_row {
                    self.selected_row += 1;
                    // Check if we need to scroll the view down
                    if self.selected_row >= self.scroll_y + self.last_visible_rows {
                        self.scroll_y =
                            self.selected_row.saturating_sub(self.last_visible_rows - 1);
                        self.fetch_table_data_preserve_selection(context); // Reload data with new offset
                    }
                }
                StateTransition::Stay
            }
            KeyCode::Char('i') => {
                // Enter inspector mode (replace current main panel content)
                StateTransition::Replace(Box::new(super::TableInspector::new()))
            }
            KeyCode::Char('r') => {
                // Refresh table data
                self.fetch_table_data(context);
                StateTransition::Stay
            }
            KeyCode::Char('m') => {
                // Enter modify mode for column operations
                // Current selected column is already in global state
                StateTransition::Push(Box::new(super::ColumnReorderMode::new()))
            }
            KeyCode::Enter => {
                // Toggle column expansion
                self.toggle_column_expansion();
                StateTransition::Stay
            }
            KeyCode::PageUp => {
                // Page up
                let page_size = self.last_visible_rows.saturating_sub(1).max(1);
                self.selected_row = self.selected_row.saturating_sub(page_size);
                if self.selected_row < self.scroll_y {
                    self.scroll_y = self.selected_row;
                    self.fetch_table_data_preserve_selection(context);
                }
                StateTransition::Stay
            }
            KeyCode::PageDown => {
                // Page down
                let max_row = if let Some(total) = self.total_row_count {
                    total.saturating_sub(1)
                } else {
                    self.table_data
                        .as_ref()
                        .map(|d| d.rows.len().saturating_sub(1))
                        .unwrap_or(0)
                };

                let page_size = self.last_visible_rows.saturating_sub(1).max(1);
                self.selected_row = (self.selected_row + page_size).min(max_row);
                if self.selected_row >= self.scroll_y + self.last_visible_rows {
                    self.scroll_y = self.selected_row.saturating_sub(self.last_visible_rows - 1);
                    self.fetch_table_data_preserve_selection(context);
                }
                StateTransition::Stay
            }
            KeyCode::Home => {
                // Go to first row
                self.selected_row = 0;
                if self.scroll_y > 0 {
                    self.scroll_y = 0;
                    self.fetch_table_data_preserve_selection(context);
                }
                StateTransition::Stay
            }
            KeyCode::End => {
                // Go to last row
                let max_row = if let Some(total) = self.total_row_count {
                    total.saturating_sub(1)
                } else {
                    self.table_data
                        .as_ref()
                        .map(|d| d.rows.len().saturating_sub(1))
                        .unwrap_or(0)
                };

                self.selected_row = max_row;
                let new_scroll_y = max_row.saturating_sub(self.last_visible_rows.saturating_sub(1));
                if new_scroll_y != self.scroll_y {
                    self.scroll_y = new_scroll_y;
                    self.fetch_table_data_preserve_selection(context);
                }
                StateTransition::Stay
            }
            KeyCode::Char('a') => {
                // Add column to sort chain (ascending)
                if let Some(column_name) = &context.global_state.current_selected_column {
                    context
                        .global_state
                        .toggle_column_sort(column_name.clone(), true);
                    self.fetch_table_data(context); // Refresh data with new sort
                }
                StateTransition::Stay
            }
            KeyCode::Char('A') => {
                // Add column to sort chain (descending)
                if let Some(column_name) = &context.global_state.current_selected_column {
                    context
                        .global_state
                        .toggle_column_sort(column_name.clone(), false);
                    self.fetch_table_data(context); // Refresh data with new sort
                }
                StateTransition::Stay
            }
            KeyCode::Char('c') => {
                // Clear all sorting
                context.global_state.clear_sort();
                self.fetch_table_data(context); // Refresh data without sorting
                StateTransition::Stay
            }
            KeyCode::Char('f') => {
                // Start column filter
                if let Some(column_name) = &context.global_state.current_selected_column {
                    context
                        .global_state
                        .start_column_filter(column_name.clone());
                    StateTransition::Push(Box::new(super::ColumnFilterInput::new()))
                } else {
                    StateTransition::Stay
                }
            }
            KeyCode::Char('F') => {
                // Clear filter on selected column
                if let Some(column_name) = context.global_state.current_selected_column.clone() {
                    context.global_state.clear_column_filter(&column_name);
                    self.fetch_table_data(context); // Refresh data without filter
                }
                StateTransition::Stay
            }
            KeyCode::Char('v') => {
                // Start view creation
                if self.table_data.is_some() {
                    context.global_state.start_view_name_input();
                    StateTransition::Push(Box::new(super::ViewNameInput::new()))
                } else {
                    StateTransition::Stay
                }
            }
            KeyCode::Char('H') => {
                // Move to leftmost visible column
                context.global_state.current_selected_column =
                    self.get_first_visible_column(context);
                if let Some(column_index) = self.get_selected_column_index(context) {
                    self.ensure_column_visible(column_index);
                }
                StateTransition::Stay
            }
            KeyCode::Char('L') => {
                // Move to rightmost visible column
                context.global_state.current_selected_column =
                    self.get_last_visible_column(context);
                if let Some(column_index) = self.get_selected_column_index(context) {
                    self.ensure_column_visible(column_index);
                }
                StateTransition::Stay
            }
            KeyCode::Char('J') => {
                // Move to bottom of table (same as End key)
                let max_row = if let Some(total) = self.total_row_count {
                    total.saturating_sub(1)
                } else {
                    self.table_data
                        .as_ref()
                        .map(|d| d.rows.len().saturating_sub(1))
                        .unwrap_or(0)
                };

                self.selected_row = max_row;
                let new_scroll_y = max_row.saturating_sub(self.last_visible_rows.saturating_sub(1));
                if new_scroll_y != self.scroll_y {
                    self.scroll_y = new_scroll_y;
                    self.fetch_table_data_preserve_selection(context);
                }
                StateTransition::Stay
            }
            KeyCode::Char('K') => {
                // Move to top of table
                self.selected_row = 0;
                if self.scroll_y > 0 {
                    self.scroll_y = 0;
                    self.fetch_table_data_preserve_selection(context); // Reload from beginning but preserve column selection
                }
                StateTransition::Stay
            }
            _ => StateTransition::Stay,
        }
    }

    fn on_enter(&mut self, context: &mut StateContext) -> Result<()> {
        tracing::debug!("Entered TableDataViewer");
        // Only load table data if there's a selected table
        if context.global_state.selected_table.is_some() {
            // Use preserve selection method to maintain column selection and viewport
            self.fetch_table_data_preserve_selection(context);
            
            // Auto-detect and handle hidden column scenarios
            // This handles cases where columns were hidden via ColumnReorderMode
            self.handle_column_reorder_completion(context);
        } else {
            tracing::debug!("No table selected, skipping data fetch");
            self.table_data = None;
        }
        Ok(())
    }

    fn debug_name(&self) -> &'static str {
        "TableDataViewer"
    }
}

impl TableDataViewer {
    /// Helper methods for column navigation (following main branch pattern)

    /// Get the next visible column after the current one
    fn get_next_visible_column_with_context(
        &self,
        current_column: &str,
        context: &StateContext,
    ) -> Option<String> {
        if let Some(ref data) = self.table_data {
            if let Some(current_idx) = data.columns.iter().position(|name| name == current_column) {
                for idx in (current_idx + 1)..data.columns.len() {
                    if let Some(name) = data.columns.get(idx) {
                        if !self.is_column_hidden_by_name(name, context) {
                            return Some(name.clone());
                        }
                    }
                }
            }
        }
        None
    }

    /// Get the previous visible column before the current one
    fn get_prev_visible_column_with_context(
        &self,
        current_column: &str,
        context: &StateContext,
    ) -> Option<String> {
        if let Some(ref data) = self.table_data {
            if let Some(current_idx) = data.columns.iter().position(|name| name == current_column) {
                for idx in (0..current_idx).rev() {
                    if let Some(name) = data.columns.get(idx) {
                        if !self.is_column_hidden_by_name(name, context) {
                            return Some(name.clone());
                        }
                    }
                }
            }
        }
        None
    }

    /// Get the first visible column
    fn get_first_visible_column(&self, context: &StateContext) -> Option<String> {
        if let Some(ref data) = self.table_data {
            for name in &data.columns {
                if !self.is_column_hidden_by_name(name, context) {
                    return Some(name.clone());
                }
            }
        }
        None
    }

    /// Get the last visible column
    fn get_last_visible_column(&self, context: &StateContext) -> Option<String> {
        if let Some(ref data) = self.table_data {
            for name in data.columns.iter().rev() {
                if !self.is_column_hidden_by_name(name, context) {
                    return Some(name.clone());
                }
            }
        }
        None
    }

    /// Get the index of the currently selected column
    fn get_selected_column_index(&self, context: &StateContext) -> Option<usize> {
        if let (Some(ref selected_col), Some(ref data)) = (&context.global_state.current_selected_column, &self.table_data)
        {
            data.columns.iter().position(|name| name == selected_col)
        } else {
            None
        }
    }

    /// Check if a column is hidden by name with context
    fn is_column_hidden_by_name(&self, column_name: &str, context: &StateContext) -> bool {
        if let Some(table_name) = &context.global_state.selected_table {
            if let Some(hidden_columns) = context.global_state.hidden_columns.get(table_name) {
                return hidden_columns.contains(column_name);
            }
        }
        false
    }

    /// Adjust column selection after column visibility changes
    /// If the current column is hidden, move to the left column, or right if at the start
    pub fn adjust_selection_after_column_hide(&mut self, context: &mut StateContext) {
        // Check if currently selected column is hidden
        if let Some(ref current_col) = context.global_state.current_selected_column.clone() {
            if self.is_column_hidden_by_name(current_col, context) {
                // Current column is hidden, need to find a new selection

                // First try to move left (previous visible column)
                if let Some(prev_col) =
                    self.get_prev_visible_column_with_context(current_col, context)
                {
                    context.global_state.current_selected_column = Some(prev_col);
                } else {
                    // No previous visible column, try to move right (next visible column)
                    if let Some(next_col) =
                        self.get_next_visible_column_with_context(current_col, context)
                    {
                        context.global_state.current_selected_column = Some(next_col);
                    } else {
                        // No visible columns at all, select first visible column if any exist
                        context.global_state.current_selected_column = self.get_first_visible_column(context);
                    }
                }

                // Ensure the new column is visible in viewport
                if let Some(column_index) = self.get_selected_column_index(context) {
                    self.ensure_column_visible(column_index);
                }
            }
        } else {
            // No current selection, select first visible column
            context.global_state.current_selected_column = self.get_first_visible_column(context);
        }
    }
}

impl Default for TableDataViewer {
    fn default() -> Self {
        Self::new()
    }
}
