use std::time::Duration;

use chrono::Local;
use ratatui::crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind,
};
use ratatui::crossterm::execute;
use ratatui::DefaultTerminal;
use std::io::stdout;

mod app;
pub mod config;
pub mod keymap;
mod model;
mod storage;
pub mod store;
pub mod theme;
mod ui;

use app::App;

fn time_until_next_midnight() -> Duration {
    let now = Local::now();
    let tomorrow = now.date_naive().succ_opt().unwrap_or(now.date_naive());
    if let Some(naive_midnight) = tomorrow.and_hms_opt(0, 0, 0) {
        let next_midnight = match naive_midnight.and_local_timezone(now.timezone()) {
            chrono::LocalResult::Single(dt) => Some(dt),
            chrono::LocalResult::Ambiguous(dt, _) => Some(dt),
            chrono::LocalResult::None => None,
        };
        if let Some(next_midnight) = next_midnight {
            if let Ok(duration) = (next_midnight - now).to_std() {
                return duration.clamp(Duration::from_secs(1), Duration::from_secs(60));
            }
        }
    }
    Duration::from_secs(60)
}

fn run_app(terminal: &mut DefaultTerminal, mut app: App) -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(size) = terminal.size() {
        app.on_resize(size.width, size.height);
    }

    terminal.draw(|f| ui::render(&mut app, f))?;

    loop {
        let timeout = time_until_next_midnight();
        let mut needs_redraw = false;

        if event::poll(timeout)? {
            match event::read()? {
                Event::Resize(w, h) => {
                    app.on_resize(w, h);
                    needs_redraw = true;
                }
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    if app.handle_key(key) {
                        break;
                    }
                    needs_redraw = true;
                }
                Event::Mouse(mouse_event) if app.handle_mouse(mouse_event) => {
                    needs_redraw = true;
                }
                _ => {}
            }
        }

        let today = Local::now().date_naive();
        if app.tick(today) {
            needs_redraw = true;
        }

        if needs_redraw {
            terminal.draw(|f| ui::render(&mut app, f))?;
        }
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (config, warnings) = config::Config::load();
    let mouse_enabled = config.ui.mouse;
    let app = App::load_or_exit_with_config(config, warnings);

    // Install panic hook to restore terminal mode safely even on panic
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        if mouse_enabled {
            let _ = execute!(stdout(), DisableMouseCapture);
        }
        ratatui::restore();
        original_hook(panic_info);
    }));

    if mouse_enabled {
        let _ = execute!(stdout(), EnableMouseCapture);
    }

    let mut terminal = ratatui::init();
    let res = run_app(&mut terminal, app);
    ratatui::restore();

    if mouse_enabled {
        let _ = execute!(stdout(), DisableMouseCapture);
    }

    if let Err(e) = res {
        eprintln!("Application error: {e}");
        std::process::exit(1);
    }

    Ok(())
}
