use crate::app::state::{ApplicationState, Notification};
use crate::db::query::{QueryResult, SortColumnSpec};
use std::collections::{HashMap, HashSet};

pub struct AppContext {
    pub current_database: Option<String>,
    pub selected_table: Option<String>,
    pub notifications: Vec<Notification>,
    // Table data state - moved from ApplicationState
    pub table_data: Option<QueryResult>,
    // Table query state for the state pattern
    pub sort_columns: Vec<SortColumnSpec>,
    pub column_filters: HashMap<String, String>, // column_name -> filter_text
    pub column_order: HashMap<String, Vec<String>>, // table_name -> ordered_column_names
    pub hidden_columns: HashMap<String, HashSet<String>>, // table_name -> hidden_column_names
    pub original_column_names: Vec<String>, // cached for current table
    // Keep reference to legacy state during transition
    pub legacy_state: Option<ApplicationState>,
    // Flash effect state for panel transitions
    pub panel_flash_timer: Option<std::time::Instant>,
    pub flash_duration_ms: u64,
}

impl AppContext {
    pub fn new() -> Self {
        Self {
            current_database: None,
            selected_table: None,
            notifications: Vec::new(),
            table_data: None,
            sort_columns: Vec::new(),
            column_filters: HashMap::new(),
            column_order: HashMap::new(),
            hidden_columns: HashMap::new(),
            original_column_names: Vec::new(),
            legacy_state: None,
            panel_flash_timer: None,
            flash_duration_ms: 1000, // 1.0 second, matching ApplicationState default
        }
    }

    pub fn add_notification(&mut self, notification: Notification) {
        self.notifications.push(notification);
    }

    pub fn show_success(&mut self, message: String) {
        self.add_notification(Notification::success(message));
    }

    pub fn show_error(&mut self, message: String) {
        self.add_notification(Notification::error(message));
    }

    pub fn show_info(&mut self, message: String) {
        self.add_notification(Notification::info(message));
    }

    pub fn remove_expired_notifications(&mut self) {
        self.notifications.retain(|n| !n.is_expired());
    }

    pub fn is_panel_flashing(&self) -> bool {
        if let Some(start_time) = self.panel_flash_timer {
            start_time.elapsed().as_millis() < self.flash_duration_ms as u128
        } else {
            false
        }
    }
}