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
        
        let mut stmt = self.connection.prepare(sql)?;
        let column_count = stmt.column_count();
        
        // Get column names
        let mut columns = Vec::new();
        for i in 0..column_count {
            if let Ok(name) = stmt.column_name(i) {
                columns.push(name.to_string());
            }
        }
        
        // Execute query and collect results
        let rows = stmt.query_map([], |row| {
            let mut row_data = Vec::new();
            for i in 0..column_count {
                let value = format_column_value(row, i)?;
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

fn format_column_value(row: &Row, index: usize) -> Result<String, duckdb::Error> {
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
                    // Try as float
                    match row.get::<_, Option<f64>>(index) {
                        Ok(Some(f)) => Ok(f.to_string()),
                        Ok(None) => Ok("NULL".to_string()),
                        Err(_) => {
                            // Try as boolean
                            match row.get::<_, Option<bool>>(index) {
                                Ok(Some(b)) => Ok(b.to_string()),
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