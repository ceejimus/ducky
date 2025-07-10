use std::path::{Path, PathBuf};
use std::fs;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph, Clear},
    Frame,
};

#[derive(Debug, Clone)]
pub struct FileItem {
    pub path: PathBuf,
    pub name: String,
    pub is_directory: bool,
    pub is_database: bool,
}

impl FileItem {
    pub fn new(path: PathBuf) -> Self {
        let name = path.file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        
        let is_directory = path.is_dir();
        let is_database = !is_directory && Self::is_database_file(&path);
        
        Self {
            path,
            name,
            is_directory,
            is_database,
        }
    }
    
    fn is_database_file(path: &Path) -> bool {
        if let Some(extension) = path.extension() {
            let ext = extension.to_string_lossy().to_lowercase();
            matches!(ext.as_str(), "db" | "duckdb" | "sqlite" | "sqlite3")
        } else {
            false
        }
    }
    
    pub fn get_display_name(&self) -> String {
        if self.is_directory {
            format!("📁 {}/", self.name)
        } else if self.is_database {
            format!("🗃️  {}", self.name)
        } else {
            format!("📄 {}", self.name)
        }
    }
    
    #[allow(dead_code)] // Future use for Phase 2+ features
    pub fn get_icon(&self) -> &'static str {
        if self.is_directory {
            "📁"
        } else if self.is_database {
            "🗃️"
        } else {
            "📄"
        }
    }
}

pub struct FileBrowser {
    current_path: PathBuf,
    items: Vec<FileItem>,
    selected_index: usize,
    show_hidden: bool,
}

impl FileBrowser {
    pub fn new() -> Result<Self> {
        let current_path = std::env::current_dir()?;
        let mut browser = Self {
            current_path,
            items: Vec::new(),
            selected_index: 0,
            show_hidden: false,
        };
        
        browser.refresh()?;
        Ok(browser)
    }
    
    pub fn refresh(&mut self) -> Result<()> {
        self.items.clear();
        self.selected_index = 0;
        
        // Add parent directory entry if not at root
        if let Some(parent) = self.current_path.parent() {
            self.items.push(FileItem {
                path: parent.to_path_buf(),
                name: "..".to_string(),
                is_directory: true,
                is_database: false,
            });
        }
        
        // Read directory contents
        let entries = fs::read_dir(&self.current_path)?;
        let mut files = Vec::new();
        let mut dirs = Vec::new();
        
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            
            // Skip hidden files unless show_hidden is true
            if !self.show_hidden {
                if let Some(name) = path.file_name() {
                    if name.to_string_lossy().starts_with('.') {
                        continue;
                    }
                }
            }
            
            let item = FileItem::new(path);
            if item.is_directory {
                dirs.push(item);
            } else {
                files.push(item);
            }
        }
        
        // Sort directories and files separately
        dirs.sort_by(|a, b| a.name.cmp(&b.name));
        files.sort_by(|a, b| a.name.cmp(&b.name));
        
        // Add directories first, then files
        self.items.extend(dirs);
        self.items.extend(files);
        
        Ok(())
    }
    
    pub fn navigate_to(&mut self, path: &Path) -> Result<()> {
        if path.is_dir() {
            self.current_path = path.to_path_buf();
            self.refresh()?;
        }
        Ok(())
    }
    
    pub fn navigate_up(&mut self) -> Result<()> {
        if let Some(parent) = self.current_path.parent() {
            let parent_path = parent.to_path_buf();
            self.navigate_to(&parent_path)?;
        }
        Ok(())
    }
    
    pub fn get_selected_item(&self) -> Option<&FileItem> {
        self.items.get(self.selected_index)
    }
    
    #[allow(dead_code)] // Future use for Phase 2+ features
    pub fn get_selected_path(&self) -> Option<PathBuf> {
        self.get_selected_item().map(|item| item.path.clone())
    }
    
    pub fn handle_key(&mut self, key: KeyEvent) -> Result<Option<PathBuf>> {
        match key.code {
            KeyCode::Up => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
            }
            KeyCode::Down => {
                if self.selected_index < self.items.len().saturating_sub(1) {
                    self.selected_index += 1;
                }
            }
            KeyCode::Enter => {
                if let Some(item) = self.get_selected_item() {
                    if item.is_directory {
                        let path = item.path.clone();
                        self.navigate_to(&path)?;
                    } else if item.is_database {
                        return Ok(Some(item.path.clone()));
                    }
                }
            }
            KeyCode::Char('h') => {
                self.show_hidden = !self.show_hidden;
                self.refresh()?;
            }
            KeyCode::Char('r') => {
                self.refresh()?;
            }
            KeyCode::Backspace => {
                self.navigate_up()?;
            }
            _ => {}
        }
        
        Ok(None)
    }
    
    pub fn render(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Path display
                Constraint::Min(0),     // File list
                Constraint::Length(3),  // Help
            ])
            .split(area);
        
        // Current path
        let path_display = Paragraph::new(format!("Path: {}", self.current_path.display()))
            .style(Style::default().fg(Color::Cyan))
            .block(Block::default().borders(Borders::ALL).title("Current Directory"));
        f.render_widget(path_display, chunks[0]);
        
        // File list
        let items: Vec<ListItem> = self.items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let style = if i == self.selected_index {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else if item.is_database {
                    Style::default().fg(Color::Green)
                } else if item.is_directory {
                    Style::default().fg(Color::Blue)
                } else {
                    Style::default().fg(Color::White)
                };
                
                ListItem::new(item.get_display_name()).style(style)
            })
            .collect();
        
        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Files"))
            .style(Style::default().fg(Color::White));
        
        f.render_widget(list, chunks[1]);
        
        // Help text
        let help_text = if self.show_hidden {
            "↑↓: Navigate | Enter: Select/Open | Backspace: Up | h: Hide hidden files | r: Refresh | Esc: Cancel"
        } else {
            "↑↓: Navigate | Enter: Select/Open | Backspace: Up | h: Show hidden files | r: Refresh | Esc: Cancel"
        };
        
        let help = Paragraph::new(help_text)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title("Help"));
        
        f.render_widget(help, chunks[2]);
    }
}

impl Default for FileBrowser {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            current_path: PathBuf::from("."),
            items: Vec::new(),
            selected_index: 0,
            show_hidden: false,
        })
    }
}

pub fn render_file_browser_popup(f: &mut Frame, area: Rect, browser: &FileBrowser) {
    let popup_area = centered_rect(80, 80, area);
    
    // Clear the area
    f.render_widget(Clear, popup_area);
    
    // Render the file browser
    let block = Block::default()
        .title("Select Database File")
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Black));
    
    f.render_widget(block, popup_area);
    
    let inner_area = Layout::default()
        .margin(1)
        .constraints([Constraint::Percentage(100)])
        .split(popup_area)[0];
    
    browser.render(f, inner_area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}