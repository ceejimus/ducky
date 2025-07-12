use std::collections::HashMap;
use std::path::Path;
use anyhow::{Result, Context};
use duckdb::Connection;

use super::{DatabaseInfo, test_connection, get_table_list};

pub struct DatabaseManager {
    connections: HashMap<String, Connection>,
    databases: Vec<DatabaseInfo>,
    current_database: Option<String>,
}

impl DatabaseManager {
    pub fn new() -> Self {
        Self {
            connections: HashMap::new(),
            databases: Vec::new(),
            current_database: None,
        }
    }

    pub fn add_database(&mut self, name: String, path: String) -> Result<()> {
        // Test connection first
        test_connection(&path)?;
        
        // Open connection
        let conn = if path.is_empty() || path == ":memory:" {
            Connection::open_in_memory()
        } else {
            Connection::open(&path)
        }.context("Failed to open database connection")?;
        
        // In-memory databases start empty (no sample data)
        
        // Get table information - propagate errors to user
        let tables = get_table_list(&conn)
            .with_context(|| format!("Failed to get table list for database '{name}'"))?;
        
        let mut db_info = DatabaseInfo::new(name.clone(), path.clone());
        db_info.tables = tables;
        
        // Store connection and database info
        self.connections.insert(name.clone(), conn);
        self.databases.push(db_info);
        
        Ok(())
    }

    pub fn remove_database(&mut self, name: &str) -> Result<()> {
        if let Some(conn) = self.connections.remove(name) {
            drop(conn);
            self.databases.retain(|db| db.name != name);
            
            if self.current_database.as_ref() == Some(&name.to_string()) {
                self.current_database = None;
            }
            
            Ok(())
        } else {
            Err(anyhow::anyhow!("Database {} not found", name))
        }
    }

    pub fn set_current_database(&mut self, name: &str) -> Result<()> {
        if self.connections.contains_key(name) {
            self.current_database = Some(name.to_string());
            Ok(())
        } else {
            Err(anyhow::anyhow!("Database {} not found", name))
        }
    }

    pub fn get_current_connection(&self) -> Option<&Connection> {
        self.current_database.as_ref()
            .and_then(|name| self.connections.get(name))
    }

    #[allow(dead_code)]
    pub fn get_connection(&self, name: &str) -> Option<&Connection> {
        self.connections.get(name)
    }

    pub fn get_databases(&self) -> &[DatabaseInfo] {
        &self.databases
    }

    pub fn get_current_database(&self) -> Option<&str> {
        self.current_database.as_deref()
    }

    #[allow(dead_code)]
    pub fn refresh_database(&mut self, name: &str) -> Result<()> {
        if let Some(conn) = self.connections.get(name) {
            let tables = get_table_list(conn)?;
            
            if let Some(db_info) = self.databases.iter_mut().find(|db| db.name == name) {
                db_info.tables = tables;
            }
            
            Ok(())
        } else {
            Err(anyhow::anyhow!("Database {} not found", name))
        }
    }

    pub fn get_database_info(&self, name: &str) -> Option<&DatabaseInfo> {
        self.databases.iter().find(|db| db.name == name)
    }

    pub fn initialize_default_databases(&mut self) -> Result<()> {
        // Add in-memory database
        self.add_database("memory".to_string(), ":memory:".to_string())?;
        
        // Set it as current
        self.set_current_database("memory")?;
        
        Ok(())
    }

    #[allow(dead_code)]
    pub fn connect_to_file(&mut self, path: &Path) -> Result<String> {
        let name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        
        let path_str = path.to_string_lossy().to_string();
        
        // Check if database already exists
        if self.connections.contains_key(&name) {
            return Err(anyhow::anyhow!("Database {} already connected", name));
        }
        
        self.add_database(name.clone(), path_str)?;
        Ok(name)
    }

    pub fn remove_table(&mut self, table_name: &str) -> Result<()> {
        if let Some(current_db) = &self.current_database {
            let current_db_name = current_db.clone();
            if let Some(conn) = self.connections.get(&current_db_name) {
                // Execute DROP TABLE command
                let sql = format!("DROP TABLE IF EXISTS {table_name}");
                conn.execute(&sql, [])
                    .with_context(|| format!("Failed to drop table '{table_name}'"))?;
                
                // Refresh database info to update table list
                self.refresh_database(&current_db_name)?;
                
                Ok(())
            } else {
                Err(anyhow::anyhow!("No connection to current database"))
            }
        } else {
            Err(anyhow::anyhow!("No current database selected"))
        }
    }

    pub fn save_database_to_file(&self, database_name: &str, file_path: &str) -> Result<()> {
        if let Some(source_conn) = self.connections.get(database_name) {
            // Use DuckDB's COPY TO command which is much simpler and more efficient
            // First get all table names using DuckDB system tables
            let tables_query = "SHOW TABLES";
            let mut stmt = source_conn.prepare(tables_query)?;
            let mut rows = stmt.query([])?;
            
            let mut table_names = Vec::new();
            while let Some(row) = rows.next()? {
                let table_name: String = row.get(0)?;
                table_names.push(table_name);
            }
            
            if table_names.is_empty() {
                return Err(anyhow::anyhow!("No tables found in database '{}'", database_name));
            }
            
            // Create target database file by opening a connection to it
            let target_conn = Connection::open(file_path)
                .with_context(|| format!("Failed to create target database file '{file_path}'"))?;
            
            // For each table, copy schema and data using DuckDB-specific approach
            for table_name in table_names {
                // Get table schema using DuckDB DESCRIBE
                let describe_sql = format!("DESCRIBE {table_name}");
                let mut describe_stmt = source_conn.prepare(&describe_sql)?;
                let mut describe_rows = describe_stmt.query([])?;
                
                let mut columns = Vec::new();
                while let Some(row) = describe_rows.next()? {
                    let col_name: String = row.get(0)?; // column_name
                    let col_type: String = row.get(1)?; // column_type
                    // DuckDB DESCRIBE gives us: column_name, column_type, null, key, default, extra
                    let col_def = format!("{col_name} {col_type}");
                    columns.push(col_def);
                }
                
                if !columns.is_empty() {
                    // Create table in target database
                    let create_table_sql = format!(
                        "CREATE TABLE {} ({})", 
                        table_name, 
                        columns.join(", ")
                    );
                    target_conn.execute(&create_table_sql, [])?;
                    
                    // Copy data using DuckDB's efficient bulk copy
                    // Export to CSV temporarily then import
                    let temp_csv = format!("/tmp/ducky_export_{table_name}.csv");
                    
                    // Export table to CSV
                    let copy_to_sql = format!("COPY {table_name} TO '{temp_csv}' (FORMAT CSV, HEADER)");
                    source_conn.execute(&copy_to_sql, [])?;
                    
                    // Import CSV to target database
                    let copy_from_sql = format!("COPY {table_name} FROM '{temp_csv}' (FORMAT CSV, HEADER)");
                    target_conn.execute(&copy_from_sql, [])?;
                    
                    // Clean up temporary file
                    std::fs::remove_file(&temp_csv).ok(); // Ignore errors on cleanup
                }
            }
            
            Ok(())
        } else {
            Err(anyhow::anyhow!("Database '{}' not found", database_name))
        }
    }
}

impl Default for DatabaseManager {
    fn default() -> Self {
        Self::new()
    }
}