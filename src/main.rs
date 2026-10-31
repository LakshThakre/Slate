use std::io::{stdout, IsTerminal};
use std::time::Duration;

use chrono::Local;
use clap::Parser;
use ratatui::crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind,
};
use ratatui::crossterm::execute;
use ratatui::DefaultTerminal;
use slate::app::App;
use slate::cli::Cli;

fn run_app(terminal: &mut DefaultTerminal, mut app: App) -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(size) = terminal.size() {
        app.on_resize(size.width, size.height);
    }

    terminal.draw(|f| slate::ui::render(&mut app, f))?;

    loop {
        let now = Local::now();
        let interval = Duration::from_millis(app.config.storage.reload_interval_ms);
        let timeout = slate::watch::next_timeout(now, interval, app.config.storage.live_reload);
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

        if app.config.storage.live_reload
            && app.check_for_external_changes(std::time::Instant::now())
        {
            needs_redraw = true;
        }

        let today = Local::now().date_naive();
        if app.tick(today) {
            needs_redraw = true;
        }

        if needs_redraw {
            terminal.draw(|f| slate::ui::render(&mut app, f))?;
        }
    }
    Ok(())
}

fn main() {
    let cli = Cli::parse();

    let data_path = match slate::resolve_data_path() {
        Ok(path) => path,
        Err(e) => {
            eprintln!("slate: {e}");
            std::process::exit(1);
        }
    };

    let (config, warnings) = slate::config::Config::load_from_path_or_env(
        cli.config.as_deref(),
        &slate::config::SystemEnv,
    );

    match cli.command {
        None => {
            if !stdout().is_terminal() {
                eprintln!("slate: not a terminal (try \"slate list\")");
                std::process::exit(1);
            }

            let mouse_enabled = config.ui.mouse;
            let app = App::load_or_exit_with_path_and_config(data_path, config, warnings);

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
        }
        Some(cmd) => {
            let mut writer = slate::output::SafeWriter::new(stdout());
            let today = Local::now().date_naive();
            if let Err(err) = slate::commands::run(cmd, &data_path, today, &config, &mut writer) {
                eprintln!("slate: {}", err.message());
                std::process::exit(err.exit_code());
            }
        }
    }
}
