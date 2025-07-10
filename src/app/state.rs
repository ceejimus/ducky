
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppState {
    DatabaseBrowser,
    TableViewer,
    QueryEditor,
    ImportWizard,
    ExportWizard,
    Settings,
}

impl Default for AppState {
    fn default() -> Self {
        Self::DatabaseBrowser
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NavigationPanel {
    DatabaseList,
    TableList,
    MainContent,
    StatusBar,
}

impl Default for NavigationPanel {
    fn default() -> Self {
        Self::DatabaseList
    }
}

#[derive(Debug, Clone, Default)]
pub struct ApplicationState {
    pub current_state: AppState,
    pub active_panel: NavigationPanel,
    pub selected_database: Option<String>,
    pub selected_table: Option<String>,
    pub status_message: String,
    // Table creation state
    pub is_creating_table: bool,
    pub new_table_name: String,
    pub table_creation_step: TableCreationStep,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum TableCreationStep {
    #[default]
    EnteringTableName,
    SelectingFile,
    ImportingData,
}

impl ApplicationState {
    pub fn new() -> Self {
        Self {
            current_state: AppState::DatabaseBrowser,
            active_panel: NavigationPanel::DatabaseList,
            selected_database: None,
            selected_table: None,
            status_message: "Welcome to Ducky - DuckDB TUI".to_string(),
            is_creating_table: false,
            new_table_name: String::new(),
            table_creation_step: TableCreationStep::default(),
        }
    }

    pub fn set_status(&mut self, message: String) {
        self.status_message = message;
    }

    pub fn select_database(&mut self, database: String) {
        self.selected_database = Some(database);
        self.set_status(format!("Selected database: {}", self.selected_database.as_ref().unwrap()));
    }

    pub fn select_table(&mut self, table: String) {
        self.selected_table = Some(table);
        self.set_status(format!("Selected table: {}", self.selected_table.as_ref().unwrap()));
    }

    pub fn next_panel(&mut self) {
        use NavigationPanel::*;
        self.active_panel = match self.active_panel {
            DatabaseList => TableList,
            TableList => MainContent,
            MainContent => StatusBar,
            StatusBar => DatabaseList,
        };
    }

    pub fn prev_panel(&mut self) {
        use NavigationPanel::*;
        self.active_panel = match self.active_panel {
            DatabaseList => StatusBar,
            TableList => DatabaseList,
            MainContent => TableList,
            StatusBar => MainContent,
        };
    }

    // Table creation workflow methods
    pub fn start_table_creation(&mut self) {
        self.is_creating_table = true;
        self.current_state = AppState::ImportWizard;
        self.new_table_name.clear();
        self.table_creation_step = TableCreationStep::EnteringTableName;
        self.set_status("Enter table name: ".to_string());
    }

    pub fn cancel_table_creation(&mut self) {
        self.is_creating_table = false;
        self.current_state = AppState::DatabaseBrowser;
        self.new_table_name.clear();
        self.table_creation_step = TableCreationStep::EnteringTableName;
        self.set_status("Table creation cancelled".to_string());
    }

    pub fn confirm_table_name(&mut self) {
        if !self.new_table_name.trim().is_empty() {
            self.table_creation_step = TableCreationStep::SelectingFile;
            self.set_status(format!("Creating table '{}' - select file to import", self.new_table_name));
        }
    }

    pub fn add_char_to_table_name(&mut self, c: char) {
        if self.is_creating_table && self.table_creation_step == TableCreationStep::EnteringTableName {
            self.new_table_name.push(c);
        }
    }

    pub fn remove_char_from_table_name(&mut self) {
        if self.is_creating_table && self.table_creation_step == TableCreationStep::EnteringTableName {
            self.new_table_name.pop();
        }
    }

    pub fn set_importing_data(&mut self) {
        self.table_creation_step = TableCreationStep::ImportingData;
        self.set_status(format!("Importing data into table '{}'...", self.new_table_name));
    }

    pub fn complete_table_creation(&mut self, success: bool) {
        if success {
            self.set_status(format!("Successfully created table '{}'", self.new_table_name));
            self.selected_table = Some(self.new_table_name.clone());
        } else {
            self.set_status(format!("Failed to create table '{}'", self.new_table_name));
        }
        
        self.is_creating_table = false;
        self.current_state = AppState::DatabaseBrowser;
        self.new_table_name.clear();
        self.table_creation_step = TableCreationStep::EnteringTableName;
    }
}