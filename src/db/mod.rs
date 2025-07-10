use anyhow::Result;
use duckdb::Connection;
use tracing::{info, error};

pub mod connection;
pub mod query;
pub mod schema;

pub use connection::DatabaseManager;
pub use schema::TableInfo;

#[derive(Debug, Clone)]
pub struct DatabaseInfo {
    pub name: String,
    #[allow(dead_code)] // Future use for Phase 2+ features
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
        info!("Attempting to open database file: {}", path);
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
        Ok(row.get::<_, i32>(0)?)
    })?;
    
    let mut count = 0;
    for row in rows {
        let _value: i32 = row?;
        count += 1;
    }
    
    if count == 1 {
        info!("Database connection test successful for: {}", path);
        Ok(())
    } else {
        error!("Database connection test failed for: {}", path);
        Err(anyhow::anyhow!("Connection test failed"))
    }
}

#[allow(dead_code)] // Future use for Phase 2+ features
pub fn get_database_version(conn: &Connection) -> Result<String> {
    let mut stmt = conn.prepare("SELECT version()")?;
    let version = stmt.query_row([], |row| {
        Ok(row.get::<_, String>(0)?)
    })?;
    Ok(version)
}

pub fn get_table_list(conn: &Connection) -> Result<Vec<TableInfo>> {
    let mut stmt = conn.prepare(
        "SELECT table_name, table_type, column_count, estimated_size 
         FROM information_schema.tables 
         WHERE table_schema = 'main'
         ORDER BY table_name"
    )?;
    
    let rows = stmt.query_map([], |row| {
        Ok(TableInfo {
            name: row.get::<_, String>(0)?,
            table_type: row.get::<_, String>(1)?,
            column_count: row.get::<_, i32>(2)?,
            estimated_size: row.get::<_, Option<i64>>(3)?,
        })
    })?;
    
    let mut tables = Vec::new();
    for row in rows {
        tables.push(row?);
    }
    
    Ok(tables)
}

#[allow(dead_code)] // Future use for demo/testing purposes
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
    
    info!("Sample data created successfully");
    Ok(())
}