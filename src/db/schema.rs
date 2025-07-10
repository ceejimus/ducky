#![allow(dead_code)] // Phase 2+ features - Visual Data Explorer

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableInfo {
    pub name: String,
    pub table_type: String,
    pub column_count: i32,
    pub estimated_size: Option<i64>,
}

impl TableInfo {
    pub fn new(name: String) -> Self {
        Self {
            name,
            table_type: "BASE TABLE".to_string(),
            column_count: 0,
            estimated_size: None,
        }
    }

    pub fn is_view(&self) -> bool {
        self.table_type == "VIEW"
    }

    pub fn is_table(&self) -> bool {
        self.table_type == "BASE TABLE"
    }

    pub fn get_display_name(&self) -> String {
        if self.is_view() {
            format!("📊 {}", self.name)
        } else {
            format!("📋 {}", self.name)
        }
    }

    pub fn get_size_display(&self) -> String {
        match self.estimated_size {
            Some(size) => format_size(size),
            None => "Unknown".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseSchema {
    pub name: String,
    pub tables: Vec<TableInfo>,
    pub views: Vec<TableInfo>,
    pub version: String,
}

impl DatabaseSchema {
    pub fn new(name: String) -> Self {
        Self {
            name,
            tables: Vec::new(),
            views: Vec::new(),
            version: String::new(),
        }
    }

    pub fn add_table(&mut self, table: TableInfo) {
        if table.is_view() {
            self.views.push(table);
        } else {
            self.tables.push(table);
        }
    }

    pub fn get_all_tables(&self) -> Vec<&TableInfo> {
        let mut all_tables = Vec::new();
        all_tables.extend(self.tables.iter());
        all_tables.extend(self.views.iter());
        all_tables
    }

    pub fn find_table(&self, name: &str) -> Option<&TableInfo> {
        self.tables.iter()
            .chain(self.views.iter())
            .find(|table| table.name == name)
    }

    pub fn table_count(&self) -> usize {
        self.tables.len() + self.views.len()
    }
}

fn format_size(bytes: i64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;
    
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    
    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnSchema {
    pub name: String,
    pub data_type: String,
    pub is_nullable: bool,
    pub is_primary_key: bool,
    pub is_foreign_key: bool,
    pub default_value: Option<String>,
    pub comment: Option<String>,
}

impl ColumnSchema {
    pub fn new(name: String, data_type: String) -> Self {
        Self {
            name,
            data_type,
            is_nullable: true,
            is_primary_key: false,
            is_foreign_key: false,
            default_value: None,
            comment: None,
        }
    }

    pub fn get_display_type(&self) -> String {
        let mut display = self.data_type.clone();
        
        if self.is_primary_key {
            display.push_str(" PK");
        }
        if self.is_foreign_key {
            display.push_str(" FK");
        }
        if !self.is_nullable {
            display.push_str(" NOT NULL");
        }
        
        display
    }

    pub fn get_icon(&self) -> &'static str {
        match self.data_type.to_lowercase().as_str() {
            "integer" | "int" | "bigint" | "smallint" => "🔢",
            "decimal" | "numeric" | "float" | "double" => "💰",
            "varchar" | "text" | "char" | "string" => "📝",
            "boolean" | "bool" => "✅",
            "date" | "datetime" | "timestamp" => "📅",
            "blob" | "binary" => "📦",
            _ => "❓",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableSchema {
    pub table_info: TableInfo,
    pub columns: Vec<ColumnSchema>,
    pub indexes: Vec<String>,
    pub foreign_keys: Vec<String>,
}

impl TableSchema {
    pub fn new(table_info: TableInfo) -> Self {
        Self {
            table_info,
            columns: Vec::new(),
            indexes: Vec::new(),
            foreign_keys: Vec::new(),
        }
    }

    pub fn add_column(&mut self, column: ColumnSchema) {
        self.columns.push(column);
    }

    pub fn get_primary_key_columns(&self) -> Vec<&ColumnSchema> {
        self.columns.iter().filter(|col| col.is_primary_key).collect()
    }

    pub fn get_foreign_key_columns(&self) -> Vec<&ColumnSchema> {
        self.columns.iter().filter(|col| col.is_foreign_key).collect()
    }
}