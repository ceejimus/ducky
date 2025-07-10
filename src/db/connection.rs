use std::collections::HashMap;
use std::path::Path;
use anyhow::{Result, Context};
use duckdb::Connection;
use tracing::info;

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
        
        info!("Added database: {} at {}", name, path);
        Ok(())
    }

    pub fn remove_database(&mut self, name: &str) -> Result<()> {
        if let Some(conn) = self.connections.remove(name) {
            drop(conn);
            self.databases.retain(|db| db.name != name);
            
            if self.current_database.as_ref() == Some(&name.to_string()) {
                self.current_database = None;
            }
            
            info!("Removed database: {}", name);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Database {} not found", name))
        }
    }

    pub fn set_current_database(&mut self, name: &str) -> Result<()> {
        if self.connections.contains_key(name) {
            self.current_database = Some(name.to_string());
            info!("Set current database to: {}", name);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Database {} not found", name))
        }
    }

    #[allow(dead_code)] // Future use for Phase 2+ features
    pub fn get_current_connection(&self) -> Option<&Connection> {
        self.current_database.as_ref()
            .and_then(|name| self.connections.get(name))
    }

    #[allow(dead_code)] // Future use for Phase 2+ features
    pub fn get_connection(&self, name: &str) -> Option<&Connection> {
        self.connections.get(name)
    }

    pub fn get_databases(&self) -> &[DatabaseInfo] {
        &self.databases
    }

    pub fn get_current_database(&self) -> Option<&str> {
        self.current_database.as_deref()
    }

    #[allow(dead_code)] // Future use for Phase 2+ features
    pub fn refresh_database(&mut self, name: &str) -> Result<()> {
        if let Some(conn) = self.connections.get(name) {
            let tables = get_table_list(conn)?;
            
            if let Some(db_info) = self.databases.iter_mut().find(|db| db.name == name) {
                db_info.tables = tables;
                info!("Refreshed database: {}", name);
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
        
        info!("Initialized default databases");
        Ok(())
    }

    #[allow(dead_code)] // Future use for Phase 2+ features
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
}

impl Default for DatabaseManager {
    fn default() -> Self {
        Self::new()
    }
}