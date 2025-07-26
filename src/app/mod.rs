use std::io;

pub mod state;

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::time::Duration;

use crate::ui::NewApp;
use std::path::PathBuf;

pub fn run(database_path: Option<PathBuf>) -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = NewApp::new_with_database(database_path);
    let result = run_app_loop(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run_app_loop<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut NewApp,
) -> io::Result<()> {
    loop {
        // Periodic updates: notifications, debounced tasks, etc.
        app.tick();

        terminal.draw(|f| {
            if let Err(e) = app.render(f) {
                // Log render errors but continue
                eprintln!("Render error: {e}");
            }
        })?;

        // Use polling with timeout to allow notifications to auto-expire
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            break
                        }
                        _ => {
                            if !app.handle_key(key) {
                                break; // Exit if state manager requests it
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
