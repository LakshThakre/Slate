use std::time::Duration;

use ratatui::crossterm::event::{self, Event, KeyEventKind};
use ratatui::DefaultTerminal;

mod app;
mod model;
mod storage;
mod theme;
mod ui;

use app::App;

fn run_app(terminal: &mut DefaultTerminal, mut app: App) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        terminal.draw(|f| ui::render(&mut app, f))?;

        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press && app.handle_key(key) {
                    break;
                }
            }
        }
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = App::load_or_exit();

    // Install panic hook to restore terminal mode safely even on panic
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        ratatui::restore();
        original_hook(panic_info);
    }));

    let mut terminal = ratatui::init();
    let res = run_app(&mut terminal, app);
    ratatui::restore();

    if let Err(e) = res {
        eprintln!("Application error: {e}");
        std::process::exit(1);
    }

    Ok(())
}
