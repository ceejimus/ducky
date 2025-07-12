# DuckDB TUI - "Ducky"

A high-performance Terminal User Interface (TUI) for DuckDB built with Rust and Ratatui.

## Project Overview

This project creates an intuitive TUI for DuckDB that emphasizes visual data exploration and manipulation without requiring extensive SQL knowledge. The interface leverages DuckDB's excellent performance characteristics while providing a modern, keyboard-driven experience.

## Technology Stack

- **Language**: Rust
- **TUI Framework**: Ratatui (React-like architecture)
- **Database**: DuckDB (via Rust bindings)
- **Performance**: Zero-cost abstractions over DuckDB's high-performance C API

## Architecture Philosophy

- **Minimal SQL**: Avoid writing raw SQL where possible; use visual interfaces and templates
- **Query Templates**: Pre-built, parameterized queries for common operations
- **Multi-format Support**: Seamless ingestion and export across multiple data formats
- **Performance-first**: Leverage DuckDB's columnar vectorized execution
- **Keyboard-driven**: Efficient navigation without mouse dependency

## Core Features

### 1. Universal Data Ingestion
- **Supported Formats**: CSV, JSON, Parquet, TSV, compressed files (.gz, .bz2)
- **Auto-detection**: Automatic schema inference and format detection
- **Bulk Import**: Efficient loading of large datasets using DuckDB's optimized readers

### 2. Visual Data Exploration
- **Table Browser**: Navigate and filter tables/views with configurable column filters
- **Schema Inspector**: Explore table structures, data types, and statistics
- **Data Preview**: Sample data display with pagination and sorting
- **Column Expansion**: Multi-column text expansion with wrapping support

### 3. Template-based Querying
- **Query Templates**: Pre-built parameterized queries for common operations
- **Template Library**: User-defined and system templates
- **Parameter Substitution**: Visual parameter input for template execution

### 4. Multi-format Export
- **Export Formats**: CSV, JSON, Parquet with compression options
- **Configurable Output**: Custom delimiters, headers, and formatting
- **Batch Export**: Export multiple tables/views simultaneously

## Key Design Decisions

1. **Performance Priority**: Use DuckDB's strengths (vectorized execution, columnar storage)
2. **Template-first**: Reduce SQL complexity through visual interfaces and templates
3. **Format Agnostic**: Support major data formats with optimized readers/writers
4. **Keyboard-centric**: Efficient navigation without mouse dependency
5. **Extensible**: Plugin architecture for custom data sources and export formats

## Project Structure

```
src/
├── main.rs              # Application entry point
├── app/                 # Core application state and logic
├── ui/                  # Ratatui UI components
├── db/                  # DuckDB connection and query management
├── import/              # Data ingestion systems
├── export/              # Data export systems
├── templates/           # Query template engine
└── config/              # Configuration management
```

## Contributing

This project emphasizes clean, performant code with comprehensive error handling. All contributions should maintain the keyboard-driven philosophy and leverage DuckDB's performance characteristics.

## Quick Start

```bash
# Build and run
cargo run

# With specific database
cargo run -- path/to/database.db

# Run tests
cargo test

# Check for issues
cargo clippy
```

## Code Quality Standards
- **Always fix clippy warnings**: Run `cargo clippy` and address all critical warnings before submitting changes
- **Format strings**: Use inline format syntax (`format!("text {var}")`) instead of `format!("text {}", var)`
- **Error handling**: Propagate errors to users rather than silently continuing with empty data
- **Business logic separation**: Keep UI modules focused on rendering; delegate workflows to separate modules

## Memories
- When we do the TODOs let's execute them one at a time not in groups.
- After TODO list completion check and remove dead_code attributes where appropriate
- Don't run the app yourself unless you're running tests with `cargo test`. After you make changes just run `cargo check` - I'll build and test.

## Context Files

This project uses organized context files for detailed information:

- **[Current Status](.claude/context/current_status.md)** - Implementation progress and working features
- **[Architecture](.claude/context/architecture.md)** - System design and code organization
- **[TODOs](.claude/todos/roadmap.md)** - Implementation roadmap and task tracking
- **[Design Ideas](.claude/todos/design_ideas.md)** - Future enhancement concepts
- **[Development Reference](.claude/reference/development.md)** - Commands, standards, and guidelines
- **[Recent Session](.claude/sessions/2025-07-12_column_expansion.md)** - Latest development session
- **[Critical Debugging](.claude/debugging/critical_patterns.md)** - Important bug patterns and fixes

---

**Note**: When starting work, read relevant context files to understand current implementation status and any critical patterns. The main CLAUDE.md provides the foundation - detailed information is available in linked files as needed.
