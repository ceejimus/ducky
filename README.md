# 🦆 Ducky - DuckDB TUI

A high-performance Terminal User Interface (TUI) for DuckDB built with Rust and Ratatui.

## ⚠️ Work In Progress

**This project is currently in active development.** While the basic functionality is working, many features are still being implemented. The current version supports:

- ✅ Basic TUI navigation and database management
- ✅ File import (CSV, JSON) with automatic schema detection
- ✅ Interactive file browser
- 🚧 Data visualization and table browsing (planned)
- 🚧 Query templates and visual query builder (planned)

Expect frequent changes and potential breaking updates as development continues.

## 🚀 Features

- **Universal Data Ingestion**: Import CSV, JSON, and Parquet files with automatic schema detection
- **Interactive File Browser**: Navigate and select files with a keyboard-driven interface
- **Visual Data Exploration**: Browse databases and tables with an intuitive 3-panel layout
- **Template-based Querying**: Pre-built queries for common operations (coming soon)
- **High Performance**: Built on DuckDB's columnar vectorized execution engine
- **Keyboard-driven**: Efficient navigation without mouse dependency

## 🛠️ Installation

### Prerequisites

- Rust 1.70+ (install from [rustup.rs](https://rustup.rs/))

### Build from Source

```bash
git clone https://github.com/ceejimus/ducky.git
cd ducky
cargo build --release
```

## 🎯 Usage

### Basic Usage

```bash
# Run the TUI
cargo run

# Or run the optimized release version
./target/release/ducky
```

### Command Line Options

```bash
# Connect to a specific database file
ducky path/to/database.db

# Run without TUI (for testing connections)
ducky --no-interface path/to/database.db

# Enable verbose logging
ducky --verbose
```

### Key Controls

- **Tab/Shift+Tab**: Navigate between panels
- **↑/↓**: Navigate lists
- **Enter**: Select items
- **i**: Import data (create new table from file)
- **o**: Open file browser
- **n**: Create new in-memory database
- **d**: Disconnect from current database
- **h**: Show help
- **q/Esc**: Quit

## 📋 Import Workflow

1. Press **i** to start importing data
2. Enter a table name for your data
3. Press **Enter** to open the file browser
4. Select a CSV, JSON, or Parquet file
5. Data is automatically imported with schema detection

## 🏗️ Architecture

- **Language**: Rust (for performance and safety)
- **TUI Framework**: Ratatui (React-like architecture)
- **Database**: DuckDB (with bundled feature)
- **Data Processing**: Leverages DuckDB's optimized readers

## 🧪 Testing

Run the test suite:

```bash
cargo test
```

## 📊 Supported File Formats

- **CSV**: Automatic delimiter and schema detection
- **JSON**: Array and object formats
- **Parquet**: With compression support
- **Compressed files**: `.gz`, `.bz2` support (coming soon)

## 🎨 Interface Layout

```
┌─────────────────────────────────────────────────────────────┐
│                    🦆 Ducky - DuckDB TUI                    │
├─────────────────┬─────────────────┬─────────────────────────┤
│   Databases     │     Tables      │     Main Content        │
│   [1]           │     [2]         │     [3]                 │
│                 │                 │                         │
│ 🧠 memory       │ 📋 table1       │ Select a database and   │
│ 💾 mydata.db    │ 📋 table2       │ table to view data      │
│                 │                 │                         │
│                 │                 │ Navigation:             │
│                 │                 │ • Tab: Switch panels    │
│                 │                 │ • ↑↓: Navigate lists    │
│                 │                 │ • i: Import data        │
│                 │                 │ • h: Help               │
├─────────────────┴─────────────────┴─────────────────────────┤
│ Status: Ready                                               │
└─────────────────────────────────────────────────────────────┘
```

## 🗺️ Roadmap

### Phase 1: Foundation ✅
- [x] Basic TUI layout and navigation
- [x] DuckDB integration
- [x] Database management
- [x] File browser

### Phase 2: Data Ingestion 🚧
- [x] File format detection
- [x] CSV/JSON import with auto-schema
- [x] Import workflow UI
- [ ] Parquet support
- [ ] Compressed file support
- [ ] Remote data sources (HTTP, S3)

### Phase 3: Data Exploration (Planned)
- [ ] Table data viewer with pagination
- [ ] Column filtering and sorting
- [ ] Data statistics and preview
- [ ] Search functionality

### Phase 4: Query System (Planned)
- [ ] Query template engine
- [ ] Visual query builder
- [ ] Query history
- [ ] Saved queries

## 🤝 Contributing

Contributions are welcome! Please feel free to submit issues and pull requests.

## 📄 License

This project is licensed under the MIT License - see the LICENSE file for details.

## 🙏 Acknowledgments

- [DuckDB](https://duckdb.org/) for the amazing analytical database
- [Ratatui](https://ratatui.rs/) for the excellent TUI framework
- [Rust](https://www.rust-lang.org/) for the language and ecosystem