use anyhow::Result;
use duckdb::Connection;

pub mod connection;
pub mod query;
pub mod schema;

pub use connection::DatabaseManager;
pub use schema::TableInfo;

#[derive(Debug, Clone)]
pub struct DatabaseInfo {
    pub name: String,
    #[allow(dead_code)]
    pub path: String,
    pub is_memory: bool,
    pub tables: Vec<TableInfo>,
}

impl DatabaseInfo {
    pub fn new(name: String, path: String) -> Self {
        let is_memory = path == ":memory:" || path.is_empty();
        Self {
            name,
            path,
            is_memory,
            tables: Vec::new(),
        }
    }
}

pub fn test_connection(path: &str) -> Result<()> {
    use anyhow::Context;
    
    let conn = if path.is_empty() || path == ":memory:" {
        Connection::open_in_memory()
            .context("Failed to open in-memory database")
    } else {
        Connection::open(path)
            .with_context(|| {
                format!(
                    "Failed to open database file: {} (this could be due to version incompatibility, corruption, or permissions)",
                    path
                )
            })
    }?;
    
    // Test with a simple query
    let mut stmt = conn.prepare("SELECT 1 as test")?;
    let rows = stmt.query_map([], |row| {
        row.get::<_, i32>(0)
    })?;
    
    let mut count = 0;
    for row in rows {
        let _value: i32 = row?;
        count += 1;
    }
    
    if count == 1 {
        Ok(())
    } else {
        Err(anyhow::anyhow!("Connection test failed"))
    }
}

#[allow(dead_code)]
pub fn get_database_version(conn: &Connection) -> Result<String> {
    let mut stmt = conn.prepare("SELECT version()")?;
    let version = stmt.query_row([], |row| {
        row.get::<_, String>(0)
    })?;
    Ok(version)
}

// TODO: Add support for views in addition to tables
// This will be a key feature for virtual data exploration and should integrate
// seamlessly with the column reordering system since it works at the query level
pub fn get_table_list(conn: &Connection) -> Result<Vec<TableInfo>> {
    let mut stmt = conn.prepare(
        "SELECT table_name, table_type 
         FROM information_schema.tables 
         WHERE table_schema = 'main'
         ORDER BY table_name"
    )?;
    
    let rows = stmt.query_map([], |row| {
        let table_name: String = row.get(0)?;
        let table_type: String = row.get(1)?;
        
        // Get column count from information_schema.columns
        let column_count = get_column_count(conn, &table_name).unwrap_or(0);
        
        Ok(TableInfo {
            name: table_name,
            table_type,
            column_count,
            estimated_size: None, // DuckDB doesn't provide this in information_schema
        })
    })?;
    
    let mut tables = Vec::new();
    for row in rows {
        tables.push(row?);
    }
    
    Ok(tables)
}

fn get_column_count(conn: &Connection, table_name: &str) -> Result<i32> {
    let mut stmt = conn.prepare(
        "SELECT COUNT(*) FROM information_schema.columns WHERE table_name = ? AND table_schema = 'main'"
    )?;
    
    let count = stmt.query_row([table_name], |row| {
        Ok(row.get::<_, i64>(0)? as i32)
    })?;
    
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::ActionLogger;
    use crate::db::connection::DatabaseManager;

    #[test]
    fn test_get_table_list_empty_database() {
        // Test that get_table_list works on empty databases
        let conn = Connection::open_in_memory().unwrap();
        let result = get_table_list(&conn);
        assert!(result.is_ok(), "get_table_list should work on empty database");
        let tables = result.unwrap();
        assert!(tables.is_empty(), "Empty database should have no tables");
    }

    #[test]
    fn test_get_table_list_with_tables() {
        // Test that get_table_list works with actual tables
        let conn = Connection::open_in_memory().unwrap();
        
        // Create a test table
        conn.execute(
            "CREATE TABLE test_table (id INTEGER, name VARCHAR)",
            []
        ).unwrap();
        
        let result = get_table_list(&conn);
        assert!(result.is_ok(), "get_table_list should work with tables");
        
        let tables = result.unwrap();
        assert_eq!(tables.len(), 1, "Should have exactly one table");
        assert_eq!(tables[0].name, "test_table");
        assert_eq!(tables[0].table_type, "BASE TABLE");
        assert_eq!(tables[0].column_count, 2, "Should have 2 columns");
    }

    #[test]
    fn test_database_manager_initialization() {
        // Test that DatabaseManager can initialize default databases
        let mut db_manager = DatabaseManager::new();
        let result = db_manager.initialize_default_databases();
        assert!(result.is_ok(), "Should be able to initialize default databases: {:?}", result.err());
        
        // Verify the memory database was created
        let databases = db_manager.get_databases();
        assert_eq!(databases.len(), 1, "Should have one default database");
        assert_eq!(databases[0].name, "memory");
        assert!(databases[0].is_memory, "Default database should be in-memory");
    }

    #[test]
    fn test_database_manager_add_database() {
        // Test adding new databases
        let mut db_manager = DatabaseManager::new();
        
        // Add in-memory database
        let result = db_manager.add_database("test_db".to_string(), ":memory:".to_string());
        assert!(result.is_ok(), "Should be able to add in-memory database: {:?}", result.err());
        
        let databases = db_manager.get_databases();
        assert_eq!(databases.len(), 1, "Should have one database");
        assert_eq!(databases[0].name, "test_db");
    }

    #[test]
    fn test_database_workflows_create_new_database() {
        // Test the full workflow for creating a new database
        use crate::app::state::ApplicationState;
        use crate::workflows::DatabaseWorkflows;
        
        let mut db_manager = DatabaseManager::new();
        let mut action_logger = ActionLogger::new().unwrap();
        let mut state = ApplicationState::new();
        
        let mut workflows = DatabaseWorkflows::new(
            &mut db_manager,
            &mut action_logger,
            &mut state,
        );
        
        let result = workflows.create_new_database();
        assert!(result.is_ok(), "Should be able to create new database via workflows: {:?}", result.err());
        
        // Verify database was added
        let databases = db_manager.get_databases();
        assert_eq!(databases.len(), 1, "Should have one database after creation");
        assert!(databases[0].is_memory, "Created database should be in-memory");
    }
}

#[allow(dead_code)]
pub fn create_sample_data(conn: &Connection) -> Result<()> {
    // Create sample tables for demonstration
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            email TEXT UNIQUE,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        );
        
        CREATE TABLE IF NOT EXISTS orders (
            id INTEGER PRIMARY KEY,
            user_id INTEGER,
            product_name TEXT NOT NULL,
            quantity INTEGER NOT NULL,
            price DECIMAL(10,2) NOT NULL,
            order_date TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (user_id) REFERENCES users(id)
        );
        
        CREATE TABLE IF NOT EXISTS products (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT,
            price DECIMAL(10,2) NOT NULL,
            stock_quantity INTEGER DEFAULT 0,
            category TEXT
        );
        
        INSERT OR REPLACE INTO users (id, name, email) VALUES 
            (1, 'John Doe', 'john@example.com'),
            (2, 'Jane Smith', 'jane@example.com'),
            (3, 'Bob Johnson', 'bob@example.com');
        
        INSERT OR REPLACE INTO products (id, name, description, price, stock_quantity, category) VALUES 
            (1, 'Laptop', 'High-performance laptop', 999.99, 10, 'Electronics'),
            (2, 'Mouse', 'Wireless mouse', 29.99, 50, 'Electronics'),
            (3, 'Keyboard', 'Mechanical keyboard', 89.99, 25, 'Electronics'),
            (4, 'Monitor', '4K monitor', 399.99, 15, 'Electronics');
        
        INSERT OR REPLACE INTO orders (id, user_id, product_name, quantity, price) VALUES 
            (1, 1, 'Laptop', 1, 999.99),
            (2, 2, 'Mouse', 2, 29.99),
            (3, 1, 'Keyboard', 1, 89.99),
            (4, 3, 'Monitor', 1, 399.99),
            (5, 2, 'Mouse', 1, 29.99);"
    )?;
    
    Ok(())
}