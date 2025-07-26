use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
};

use crate::state::{UIState, StateTransition, StateContext};

/// ViewNameInput handles view name input modal
pub struct ViewNameInput;

impl ViewNameInput {
    pub fn new() -> Self {
        Self
    }
}

impl UIState for ViewNameInput {
    fn render(&mut self, frame: &mut Frame, area: Rect, is_active: bool, context: &mut StateContext) -> Result<()> {
        let border_style = if is_active {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };
        
        let view_name_display = if context.global_state.is_entering_view_name {
            format!("{}|", context.global_state.new_view_name)
        } else {
            context.global_state.new_view_name.clone()
        };
        
        let table_name = context.global_state.selected_table.as_deref().unwrap_or("unknown");
        
        // Generate preview SQL for the view
        let preview_sql = generate_view_sql(table_name, context);
        
        let content = format!(
            "View Name: {}\n\nSQL Preview:\n{}\n\nPress Enter to create, Esc to cancel",
            view_name_display,
            preview_sql
        );
        
        let input_widget = Paragraph::new(content)
            .block(
                Block::default()
                    .title("Create View")
                    .borders(Borders::ALL)
                    .border_style(border_style)
            )
            .style(Style::default().fg(Color::White));
        
        frame.render_widget(input_widget, area);
        Ok(())
    }
    
    fn handle_event(&mut self, event: KeyEvent, context: &mut StateContext) -> StateTransition {
        match event.code {
            KeyCode::Esc => {
                context.global_state.cancel_view_name_input();
                StateTransition::Pop
            }
            KeyCode::Enter => {
                if !context.global_state.new_view_name.is_empty() {
                    // Create the view using the current table state
                    if let Err(e) = create_view_from_state(context) {
                        context.set_status_message(format!("Failed to create view: {}", e));
                    } else {
                        context.set_status_message(format!("View '{}' created successfully", context.global_state.new_view_name));
                        // Refresh the database to update the table list with the new view
                        let current_db = context.database_manager.get_current_database().map(|s| s.to_string());
                        if let Some(current_db) = current_db {
                            if let Err(e) = context.database_manager.refresh_database(&current_db) {
                                tracing::error!("Failed to refresh database after view creation: {}", e);
                            }
                        }
                    }
                    context.global_state.cancel_view_name_input();
                    StateTransition::Pop
                } else {
                    StateTransition::Stay
                }
            }
            KeyCode::Backspace => {
                if !context.global_state.new_view_name.is_empty() {
                    context.global_state.new_view_name.pop();
                }
                StateTransition::Stay
            }
            KeyCode::Char(c) if c.is_alphanumeric() || c == '_' => {
                context.global_state.new_view_name.push(c);
                StateTransition::Stay
            }
            _ => StateTransition::Stay,
        }
    }
    
    fn debug_name(&self) -> &'static str {
        "ViewNameInput"
    }
}

impl Default for ViewNameInput {
    fn default() -> Self {
        Self::new()
    }
}

/// Get visible column names in their custom order (if any)
fn get_visible_column_names(table_name: &str, context: &StateContext) -> Vec<String> {
    // Get all column names from the database
    if let Some(current_db) = context.database_manager.get_current_database() {
        if let Some(connection) = context.database_manager.get_connection(current_db) {
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
                    
                    // Apply custom column order if it exists
                    let ordered_columns = if let Some(custom_order) = context.global_state.get_column_order(table_name) {
                        // Start with custom order
                        let mut result = custom_order.clone();
                        
                        // Add any new columns not in the custom order (shouldn't happen normally)
                        for col_name in &all_column_names {
                            if !result.contains(col_name) {
                                result.push(col_name.clone());
                            }
                        }
                        result
                    } else {
                        // No custom order, use database order
                        all_column_names
                    };
                    
                    // Filter out hidden columns
                    if let Some(hidden_columns) = context.global_state.get_hidden_columns(table_name) {
                        ordered_columns.into_iter()
                            .filter(|col_name| !hidden_columns.contains(col_name))
                            .collect()
                    } else {
                        ordered_columns
                    }
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    }
}

/// Generate SQL for creating a view from current table state
fn generate_view_sql(table_name: &str, context: &StateContext) -> String {
    // Get visible columns in their custom order
    let visible_columns = get_visible_column_names(table_name, context);
    
    let mut sql = if visible_columns.is_empty() {
        // Fall back to SELECT * if no columns are visible or accessible
        format!("SELECT * FROM {}", table_name)
    } else {
        // Build SELECT clause with ordered visible columns
        let columns_sql = visible_columns.join(", ");
        format!("SELECT {} FROM {}", columns_sql, table_name)
    };
    
    // Add WHERE clause for filters
    if !context.global_state.column_filters.is_empty() {
        let conditions: Vec<String> = context.global_state.column_filters
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
        let order_clauses: Vec<String> = context.global_state.sort_columns
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
    
    sql
}

/// Create a view from the current state
fn create_view_from_state(context: &mut StateContext) -> Result<()> {
    let view_name = &context.global_state.new_view_name;
    let table_name = context.global_state.selected_table.as_deref().unwrap_or("unknown");
    
    let inner_sql = generate_view_sql(table_name, context);
    let create_sql = format!("CREATE VIEW {} AS {}", view_name, inner_sql);
    
    // Execute the CREATE VIEW statement
    if let Some(current_db) = context.database_manager.get_current_database() {
        if let Some(connection) = context.database_manager.get_connection(current_db) {
            match connection.execute(&create_sql, []) {
                Ok(_) => {
                    tracing::info!("Created view: {}", view_name);
                    return Ok(());
                }
                Err(e) => {
                    tracing::error!("Failed to create view '{}': {}", view_name, e);
                    return Err(anyhow::anyhow!("Database error: {}", e));
                }
            }
        }
    }
    
    Err(anyhow::anyhow!("No database connection available"))
}