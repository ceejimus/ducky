use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Rect, Constraint},
    style::{Color, Style, Modifier},
    widgets::{Block, Borders, Paragraph, Table, Row, Cell},
};

use crate::state::{UIState, StateTransition, StateContext};
use crate::db::query::QueryResult;

/// TableDataViewer manages the display and interaction with table data
pub struct TableDataViewer {
    // Scrolling and navigation state
    scroll_x: usize,
    scroll_y: usize,
    selected_row: usize,
    selected_column: Option<String>,
    page_size: usize,
    
    // Cached table data
    table_data: Option<QueryResult>,
}

impl TableDataViewer {
    pub fn new() -> Self {
        Self {
            scroll_x: 0,
            scroll_y: 0,
            selected_row: 0,
            selected_column: None,
            page_size: 20,
            table_data: None,
        }
    }
    
    /// Fetch table data for the current table
    fn fetch_table_data(&mut self, context: &mut StateContext) {
        if let Some(table_name) = &context.global_state.selected_table {
            if let Some(current_db) = context.database_manager.get_current_database() {
                if let Some(connection) = context.database_manager.get_connection(current_db) {
                    // Execute query directly using the connection
                    let sql = format!("SELECT * FROM {} LIMIT {}", table_name, self.page_size);
                    match self.execute_query_direct(connection, &sql) {
                        Ok(data) => {
                            self.table_data = Some(data);
                            context.action_logger.log_info(&format!("Loaded data for table: {}", table_name));
                        }
                        Err(e) => {
                            context.action_logger.log_error(&format!("Failed to load table data: {e}"));
                            self.table_data = None;
                        }
                    }
                }
            }
        }
    }
    
    /// Execute a query directly against the connection
    fn execute_query_direct(&self, connection: &duckdb::Connection, sql: &str) -> Result<QueryResult> {
        let start_time = std::time::Instant::now();
        
        let mut stmt = connection.prepare(sql)?;
        let column_count = stmt.column_count();
        
        // Verify we have columns (statement has a result set)
        if column_count == 0 {
            return Err(anyhow::anyhow!("Query returned no columns"));
        }
        
        // Get column names
        let mut columns = Vec::new();
        for i in 0..column_count {
            if let Ok(name) = stmt.column_name(i) {
                columns.push(name.to_string());
            } else {
                columns.push(format!("Column_{}", i + 1));
            }
        }
        
        // Execute query and collect results
        let rows = stmt.query_map([], |row| {
            let mut row_data = Vec::new();
            for i in 0..column_count {
                let value = self.format_column_value(row, i)?;
                row_data.push(value);
            }
            Ok(row_data)
        })?;
        
        let mut result_rows = Vec::new();
        for row in rows {
            result_rows.push(row?);
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
    fn format_column_value(&self, row: &duckdb::Row, index: usize) -> Result<String, duckdb::Error> {
        // Try to get the value as different types and format appropriately
        match row.get::<_, Option<String>>(index) {
            Ok(Some(s)) => Ok(s),
            Ok(None) => Ok("NULL".to_string()),
            Err(_) => {
                // Try as integer
                match row.get::<_, Option<i64>>(index) {
                    Ok(Some(i)) => Ok(i.to_string()),
                    Ok(None) => Ok("NULL".to_string()),
                    Err(_) => {
                        // Try as float - but handle NaN/Infinity safely
                        match row.get::<_, Option<f64>>(index) {
                            Ok(Some(f)) => {
                                if f.is_nan() {
                                    Ok("NaN".to_string())
                                } else if f.is_infinite() {
                                    Ok(if f.is_sign_positive() { "∞" } else { "-∞" }.to_string())
                                } else {
                                    Ok(f.to_string())
                                }
                            }
                            Ok(None) => Ok("NULL".to_string()),
                            Err(_) => {
                                // Try as boolean
                                match row.get::<_, Option<bool>>(index) {
                                    Ok(Some(b)) => Ok(b.to_string()),
                                    Ok(None) => Ok("NULL".to_string()),
                                    Err(_) => {
                                        // Try as raw bytes and format as hex
                                        match row.get::<_, Option<Vec<u8>>>(index) {
                                            Ok(Some(bytes)) => {
                                                let truncated = &bytes[..bytes.len().min(16)];
                                                let hex_str = truncated.iter().map(|b| format!("{:02x}", b)).collect::<String>();
                                                Ok(format!("0x{}", hex_str))
                                            },
                                            Ok(None) => Ok("NULL".to_string()),
                                            Err(_) => Ok("?".to_string()),
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
    fn render(&self, frame: &mut Frame, area: Rect, is_active: bool, context: &StateContext) -> Result<()> {
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
        if let Some(ref data) = self.table_data {
            let header_cells = data.columns
                .iter()
                .map(|h| Cell::from(h.as_str()).style(Style::default().add_modifier(Modifier::BOLD)))
                .collect::<Vec<_>>();
            
            let header = Row::new(header_cells)
                .style(Style::default().fg(Color::Yellow))
                .height(1);
            
            // Create rows from data, handling scrolling
            let rows: Vec<Row> = data.rows
                .iter()
                .skip(self.scroll_y)
                .take(area.height.saturating_sub(3) as usize) // Account for borders and header
                .enumerate()
                .map(|(i, row)| {
                    let cells = row.iter().map(|c| Cell::from(c.as_str())).collect::<Vec<_>>();
                    let style = if i + self.scroll_y == self.selected_row {
                        Style::default().bg(Color::DarkGray)
                    } else {
                        Style::default()
                    };
                    Row::new(cells).style(style).height(1)
                })
                .collect();
            
            // Calculate column widths (simplified - equal width for now)
            let column_count = data.columns.len();
            let column_width = if column_count > 0 {
                area.width.saturating_sub(2) / column_count as u16 // Account for borders
            } else {
                10
            };
            
            let widths = vec![Constraint::Length(column_width); column_count];
            
            let table = Table::new(rows, widths)
                .header(header)
                .block(
                    Block::default()
                        .title(title.as_str())
                        .borders(Borders::ALL)
                        .border_style(border_style)
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
                        .border_style(border_style)
                )
                .style(Style::default().fg(Color::White));
            
            frame.render_widget(placeholder, area);
        }
        
        Ok(())
    }
    
    fn handle_event(&mut self, event: KeyEvent, context: &mut StateContext) -> StateTransition {
        match event.code {
            KeyCode::Left | KeyCode::Char('h') => {
                // Focus the table panel
                StateTransition::FocusPanel(crate::state::PanelType::Table)
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
            KeyCode::Up | KeyCode::Char('k') => {
                // Move selection up
                if self.selected_row > 0 {
                    self.selected_row -= 1;
                    if self.selected_row < self.scroll_y {
                        self.scroll_y = self.selected_row;
                    }
                }
                StateTransition::Stay
            }
            KeyCode::Down | KeyCode::Char('j') => {
                // Move selection down
                if let Some(ref data) = self.table_data {
                    if self.selected_row < data.rows.len().saturating_sub(1) {
                        self.selected_row += 1;
                        // Auto-scroll if selection goes off screen
                        let visible_rows = 20; // TODO: Calculate from area
                        if self.selected_row >= self.scroll_y + visible_rows {
                            self.scroll_y = self.selected_row.saturating_sub(visible_rows - 1);
                        }
                    }
                }
                StateTransition::Stay
            }
            KeyCode::PageUp => {
                // Page up
                let page_size = 10; // TODO: Calculate from area
                self.selected_row = self.selected_row.saturating_sub(page_size);
                self.scroll_y = self.scroll_y.saturating_sub(page_size);
                StateTransition::Stay
            }
            KeyCode::PageDown => {
                // Page down
                if let Some(ref data) = self.table_data {
                    let page_size = 10; // TODO: Calculate from area
                    self.selected_row = (self.selected_row + page_size).min(data.rows.len().saturating_sub(1));
                    self.scroll_y = self.scroll_y + page_size;
                }
                StateTransition::Stay
            }
            KeyCode::Home => {
                // Go to first row
                self.selected_row = 0;
                self.scroll_y = 0;
                StateTransition::Stay
            }
            KeyCode::End => {
                // Go to last row
                if let Some(ref data) = self.table_data {
                    self.selected_row = data.rows.len().saturating_sub(1);
                    let visible_rows = 20; // TODO: Calculate from area
                    self.scroll_y = data.rows.len().saturating_sub(visible_rows).min(self.selected_row);
                }
                StateTransition::Stay
            }
            _ => StateTransition::Stay,
        }
    }
    
    fn on_enter(&mut self, context: &mut StateContext) -> Result<()> {
        context.action_logger.log_debug("Entered TableDataViewer");
        // Load table data when entering this state
        self.fetch_table_data(context);
        Ok(())
    }
    
    fn debug_name(&self) -> &'static str {
        "TableDataViewer"
    }
}

impl Default for TableDataViewer {
    fn default() -> Self {
        Self::new()
    }
}