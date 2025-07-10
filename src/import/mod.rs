use std::path::Path;
use anyhow::{Result, Context};
use duckdb::Connection;
use tracing::info;

use crate::actions::{Action, ActionLogger};

/// Supported file formats for import
#[derive(Debug, Clone, PartialEq)]
pub enum FileFormat {
    Csv,
    Json,
    Parquet,
}

impl FileFormat {
    /// Detect file format from file extension
    pub fn from_extension(path: &Path) -> Result<Self> {
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .ok_or_else(|| anyhow::anyhow!("No file extension found"))?
            .to_lowercase();

        match extension.as_str() {
            "csv" => Ok(FileFormat::Csv),
            "json" => Ok(FileFormat::Json),
            "parquet" => Ok(FileFormat::Parquet),
            _ => Err(anyhow::anyhow!("Unsupported file format: {}", extension)),
        }
    }

    /// Get the format name as a string
    pub fn as_str(&self) -> &'static str {
        match self {
            FileFormat::Csv => "CSV",
            FileFormat::Json => "JSON", 
            FileFormat::Parquet => "Parquet",
        }
    }
}

/// Import workflows for data ingestion
pub struct ImportWorkflows<'a> {
    pub connection: &'a Connection,
    pub action_logger: &'a mut ActionLogger,
}

impl<'a> ImportWorkflows<'a> {
    pub fn new(connection: &'a Connection, action_logger: &'a mut ActionLogger) -> Self {
        Self {
            connection,
            action_logger,
        }
    }

    /// Import data from a file into a table with auto-schema detection
    pub fn import_file_to_table(&mut self, file_path: &Path, table_name: &str) -> Result<()> {
        // Detect file format
        let format = FileFormat::from_extension(file_path)?;
        
        let action = Action::ImportData {
            source_path: file_path.to_string_lossy().to_string(),
            format: format.as_str().to_string(),
            destination_table: table_name.to_string(),
        };

        let tracker = self.action_logger.start_action(action);
        let result = self.execute_import_file_to_table(file_path, table_name, &format);
        self.action_logger.complete_action(tracker, &result);
        result
    }

    fn execute_import_file_to_table(
        &mut self,
        file_path: &Path,
        table_name: &str,
        format: &FileFormat,
    ) -> Result<()> {
        let file_path_str = file_path.to_string_lossy();
        
        // Build the CREATE TABLE AS SELECT query using DuckDB's auto-detection
        let query = match format {
            FileFormat::Csv => {
                format!(
                    "CREATE TABLE {} AS SELECT * FROM read_csv_auto('{}')",
                    table_name, file_path_str
                )
            }
            FileFormat::Json => {
                format!(
                    "CREATE TABLE {} AS SELECT * FROM read_json_auto('{}')",
                    table_name, file_path_str
                )
            }
            FileFormat::Parquet => {
                format!(
                    "CREATE TABLE {} AS SELECT * FROM read_parquet('{}')",
                    table_name, file_path_str
                )
            }
        };

        info!("Executing import query: {}", query);
        
        self.connection
            .execute(&query, [])
            .context("Failed to execute import query")?;

        info!("Successfully imported {} data from {} to table {}", 
              format.as_str(), file_path_str, table_name);
        
        Ok(())
    }

    /// Get schema information from a file without importing
    pub fn detect_schema(&self, file_path: &Path) -> Result<Vec<ColumnInfo>> {
        let format = FileFormat::from_extension(file_path)?;
        let file_path_str = file_path.to_string_lossy();
        
        let query = match format {
            FileFormat::Csv => {
                format!("DESCRIBE SELECT * FROM read_csv_auto('{}')", file_path_str)
            }
            FileFormat::Json => {
                format!("DESCRIBE SELECT * FROM read_json_auto('{}')", file_path_str)
            }
            FileFormat::Parquet => {
                format!("DESCRIBE SELECT * FROM read_parquet('{}')", file_path_str)
            }
        };

        let mut stmt = self.connection.prepare(&query)?;
        let rows = stmt.query_map([], |row| {
            Ok(ColumnInfo {
                name: row.get(0)?,
                data_type: row.get(1)?,
                is_null: row.get(2)?,
                key: row.get(3).ok(),
                default: row.get(4).ok(),
                extra: row.get(5).ok(),
            })
        })?;

        let mut columns = Vec::new();
        for row in rows {
            columns.push(row?);
        }

        Ok(columns)
    }
}

/// Column information from schema detection
#[derive(Debug, Clone)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub is_null: String,
    pub key: Option<String>,
    pub default: Option<String>,
    pub extra: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use duckdb::Connection;
    use crate::actions::ActionLogger;
    
    #[test]
    fn test_format_detection() {
        assert_eq!(FileFormat::from_extension(Path::new("test.csv")).unwrap(), FileFormat::Csv);
        assert_eq!(FileFormat::from_extension(Path::new("test.json")).unwrap(), FileFormat::Json);
        assert_eq!(FileFormat::from_extension(Path::new("test.parquet")).unwrap(), FileFormat::Parquet);
        assert!(FileFormat::from_extension(Path::new("test.txt")).is_err());
    }

    #[test]
    fn test_format_strings() {
        assert_eq!(FileFormat::Csv.as_str(), "CSV");
        assert_eq!(FileFormat::Json.as_str(), "JSON");
        assert_eq!(FileFormat::Parquet.as_str(), "Parquet");
    }

    #[test]
    fn test_csv_import() {
        let csv_path = Path::new("data/test_data.csv");
        if !csv_path.exists() {
            // Skip test if test data doesn't exist
            return;
        }

        let conn = Connection::open_in_memory().unwrap();
        let mut action_logger = ActionLogger::new().unwrap();
        let mut import_workflows = ImportWorkflows::new(&conn, &mut action_logger);
        
        // Test import
        let result = import_workflows.import_file_to_table(csv_path, "test_table");
        assert!(result.is_ok(), "CSV import failed: {:?}", result.err());
        
        // Verify data was imported
        let mut stmt = conn.prepare("SELECT COUNT(*) FROM test_table").unwrap();
        let count: i64 = stmt.query_row([], |row| row.get(0)).unwrap();
        assert_eq!(count, 4, "Expected 4 rows in test_table");
        
        // Verify column names
        let mut stmt = conn.prepare("SELECT name, age, city FROM test_table WHERE name = 'John'").unwrap();
        let row = stmt.query_row([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i32>(1)?,
                row.get::<_, String>(2)?
            ))
        }).unwrap();
        
        assert_eq!(row.0, "John");
        assert_eq!(row.1, 30);
        assert_eq!(row.2, "New York");
    }

    #[test]
    fn test_json_import() {
        let json_path = Path::new("data/test_data.json");
        if !json_path.exists() {
            // Skip test if test data doesn't exist
            return;
        }

        let conn = Connection::open_in_memory().unwrap();
        let mut action_logger = ActionLogger::new().unwrap();
        let mut import_workflows = ImportWorkflows::new(&conn, &mut action_logger);
        
        // Test import
        let result = import_workflows.import_file_to_table(json_path, "test_json_table");
        assert!(result.is_ok(), "JSON import failed: {:?}", result.err());
        
        // Verify data was imported
        let mut stmt = conn.prepare("SELECT COUNT(*) FROM test_json_table").unwrap();
        let count: i64 = stmt.query_row([], |row| row.get(0)).unwrap();
        assert_eq!(count, 4, "Expected 4 rows in test_json_table");
        
        // Verify column names and data
        let mut stmt = conn.prepare("SELECT name, age, city FROM test_json_table WHERE name = 'Jane'").unwrap();
        let row = stmt.query_row([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?
            ))
        }).unwrap();
        
        assert_eq!(row.0, "Jane");
        assert_eq!(row.1, 25);
        assert_eq!(row.2, "San Francisco");
    }

    #[test]
    fn test_schema_detection() {
        let csv_path = Path::new("data/test_data.csv");
        if !csv_path.exists() {
            // Skip test if test data doesn't exist
            return;
        }

        let conn = Connection::open_in_memory().unwrap();
        let mut action_logger = ActionLogger::new().unwrap();
        let import_workflows = ImportWorkflows::new(&conn, &mut action_logger);
        
        // Test schema detection
        let result = import_workflows.detect_schema(csv_path);
        assert!(result.is_ok(), "Schema detection failed: {:?}", result.err());
        
        let columns = result.unwrap();
        assert_eq!(columns.len(), 3, "Expected 3 columns");
        assert_eq!(columns[0].name, "name");
        assert_eq!(columns[1].name, "age");
        assert_eq!(columns[2].name, "city");
    }
}