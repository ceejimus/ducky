#![allow(dead_code)] // Phase 2+ features - Visual Data Explorer

use anyhow::Result;
use duckdb::{Connection, Row};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub row_count: usize,
    pub execution_time_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SortDirection {
    Ascending,
    Descending,
}

impl Default for SortDirection {
    fn default() -> Self {
        Self::Ascending
    }
}

#[derive(Debug, Clone)]
pub struct SortColumnSpec {
    pub column_name: String,
    pub direction: SortDirection,
}

/// Specification for table data queries - passed to db module to avoid direct context access
#[derive(Debug, Clone)]
pub struct TableQuerySpec {
    pub table_name: String,
    pub sort_columns: Vec<SortColumnSpec>,
    pub column_filters: HashMap<String, String>,
    pub visible_column_names: Vec<String>, // Already in desired order
    pub original_column_names: Vec<String>, // For SQL generation
    pub limit: Option<usize>,
}

impl TableQuerySpec {
    pub fn new(table_name: String) -> Self {
        Self {
            table_name,
            sort_columns: Vec::new(),
            column_filters: HashMap::new(),
            visible_column_names: Vec::new(),
            original_column_names: Vec::new(),
            limit: Some(1000), // Default limit
        }
    }
}

impl QueryResult {
    pub fn new() -> Self {
        Self {
            columns: Vec::new(),
            rows: Vec::new(),
            row_count: 0,
            execution_time_ms: 0,
        }
    }
}

pub struct QueryExecutor {
    connection: Connection,
}

impl QueryExecutor {
    pub fn new(connection: Connection) -> Self {
        Self { connection }
    }

    pub fn execute_query(&self, sql: &str) -> Result<QueryResult> {
        let start_time = std::time::Instant::now();
        
        log::debug!("Executing SQL: {}", sql);
        
        // Prepare statement and execute query
        let mut stmt = self.connection.prepare(sql)?;
        let mut rows = stmt.query([])?;
        
        // Get column count from the executed query results (not the statement)
        let column_count = rows.as_ref().unwrap().column_count();
        
        // Get column names from the query results
        let mut columns = Vec::new();
        for i in 0..column_count {
            let column_name = rows.as_ref().unwrap().column_name(i)
                .unwrap_or(&format!("column_{}", i))
                .to_string();
            columns.push(column_name);
        }
        
        // Collect all rows
        let mut result_rows = Vec::new();
        while let Some(row) = rows.next()? {
            let mut row_data = Vec::new();
            for i in 0..column_count {
                // Convert each column value to string with safer error handling
                // Try f64 first to handle NaN values, then other types
                let value = match row.get::<_, f64>(i) {
                    Ok(v) => {
                        if v.is_nan() {
                            "NaN".to_string()
                        } else if v.is_infinite() {
                            if v.is_sign_positive() { "Infinity".to_string() } else { "-Infinity".to_string() }
                        } else {
                            v.to_string()
                        }
                    },
                    Err(_) => match row.get::<_, String>(i) {
                        Ok(v) => v,
                        Err(_) => match row.get::<_, i64>(i) {
                            Ok(v) => v.to_string(),
                            Err(_) => match row.get::<_, bool>(i) {
                                Ok(v) => v.to_string(),
                                Err(_) => {
                                    // Try to get as raw value or default to NULL
                                    match row.get_ref(i) {
                                        Ok(value_ref) => format!("{value_ref:?}"),
                                        Err(_) => "NULL".to_string(),
                                    }
                                }
                            }
                        }
                    }
                };
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

    pub fn execute_query_with_limit(&self, sql: &str, limit: usize) -> Result<QueryResult> {
        let limited_sql = if sql.trim().to_lowercase().contains("limit") {
            sql.to_string()
        } else {
            format!("{sql} LIMIT {limit}")
        };
        
        self.execute_query(&limited_sql)
    }

    pub fn get_table_preview(&self, table_name: &str, limit: usize) -> Result<QueryResult> {
        let sql = format!("SELECT * FROM {table_name} LIMIT {limit}");
        self.execute_query(&sql)
    }

    pub fn get_table_preview_with_column_order(&self, table_name: &str, column_order: &[usize], column_names: &[String], limit: usize) -> Result<QueryResult> {
        // Build SELECT statement with columns in virtual order
        if column_order.is_empty() || column_names.is_empty() {
            // Fallback to regular preview if no ordering specified
            return self.get_table_preview(table_name, limit);
        }

        let mut ordered_columns = Vec::new();
        for &virtual_index in column_order {
            if virtual_index < column_names.len() {
                ordered_columns.push(column_names[virtual_index].clone());
            }
        }

        if ordered_columns.is_empty() {
            // Fallback if ordering is invalid
            return self.get_table_preview(table_name, limit);
        }

        let columns_sql = ordered_columns.join(", ");
        let sql = format!("SELECT {columns_sql} FROM {table_name} LIMIT {limit}");
        self.execute_query(&sql)
    }

    pub fn get_table_count(&self, table_name: &str) -> Result<i64> {
        let sql = format!("SELECT COUNT(*) FROM {table_name}");
        let mut stmt = self.connection.prepare(&sql)?;
        let count = stmt.query_row([], |row| row.get::<_, i64>(0))?;
        Ok(count)
    }

    pub fn fetch_table_data(&self, spec: &TableQuerySpec) -> Result<QueryResult> {
        let sql = self.build_query_sql(spec);
        self.execute_query(&sql)
    }

    fn build_query_sql(&self, spec: &TableQuerySpec) -> String {
        let mut sql = String::new();
        
        // Build SELECT clause with visible columns in desired order
        if spec.visible_column_names.is_empty() {
            sql.push_str(&format!("SELECT * FROM {}", spec.table_name));
        } else {
            let columns_sql = spec.visible_column_names.join(", ");
            sql.push_str(&format!("SELECT {} FROM {}", columns_sql, spec.table_name));
        }
        
        // Add WHERE clause for filters
        if !spec.column_filters.is_empty() {
            let mut filter_parts = Vec::new();
            for (column_name, filter_text) in &spec.column_filters {
                // Use the filter text directly as SQL (user responsibility for syntax)
                filter_parts.push(format!("{} {}", column_name, filter_text));
            }
            
            if !filter_parts.is_empty() {
                sql.push_str(&format!(" WHERE {}", filter_parts.join(" AND ")));
            }
        }
        
        // Add ORDER BY clause for sorting
        if !spec.sort_columns.is_empty() {
            let mut sort_parts = Vec::new();
            for sort_spec in &spec.sort_columns {
                let direction = match sort_spec.direction {
                    SortDirection::Ascending => "ASC",
                    SortDirection::Descending => "DESC",
                };
                sort_parts.push(format!("{} {}", sort_spec.column_name, direction));
            }
            
            if !sort_parts.is_empty() {
                sql.push_str(&format!(" ORDER BY {}", sort_parts.join(", ")));
            }
        }
        
        // Add LIMIT clause
        if let Some(limit) = spec.limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }
        
        sql
    }

    pub fn get_table_columns(&self, table_name: &str) -> Result<Vec<ColumnInfo>> {
        let sql = format!(
            "SELECT column_name, data_type, is_nullable, column_default 
             FROM information_schema.columns 
             WHERE table_name = '{table_name}' 
             ORDER BY ordinal_position"
        );
        
        let mut stmt = self.connection.prepare(&sql)?;
        let rows = stmt.query_map([], |row| {
            Ok(ColumnInfo {
                name: row.get::<_, String>(0)?,
                data_type: row.get::<_, String>(1)?,
                is_nullable: row.get::<_, String>(2)? == "YES",
                default_value: row.get::<_, Option<String>>(3)?,
            })
        })?;
        
        let mut columns = Vec::new();
        for row in rows {
            columns.push(row?);
        }
        
        Ok(columns)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub is_nullable: bool,
    pub default_value: Option<String>,
}


pub fn build_filter_query(
    table_name: &str,
    filters: &HashMap<String, String>,
    limit: Option<usize>,
) -> String {
    let mut query = format!("SELECT * FROM {table_name}");
    
    if !filters.is_empty() {
        let conditions: Vec<String> = filters
            .iter()
            .map(|(column, value)| {
                if value.contains('%') {
                    format!("{column} LIKE '{value}'")
                } else {
                    format!("{column} = '{value}'")
                }
            })
            .collect();
        
        query.push_str(&format!(" WHERE {}", conditions.join(" AND ")));
    }
    
    if let Some(limit) = limit {
        query.push_str(&format!(" LIMIT {}", limit));
    }
    
    query
}