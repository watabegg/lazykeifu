//! lazykeifu: a TUI tool that shows Git commit graphs

use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::Result;
use clap::Parser;
use crossterm::event::Event;

use lazykeifu::{
    app::App,
    config::FileView,
    debug_server,
    event::{self, EventReader, InputEvent},
    git::configure_git_extensions,
    keybindings::map_key_to_action,
    logging, mouse, tui, ui,
};

const MAINTENANCE_IDLE_PERIOD: Duration = Duration::from_millis(250);

#[derive(Parser)]
#[command(name = "lazykeifu")]
#[command(
    version,
    about = "A TUI tool to visualize Git commit graphs with branch genealogy"
)]
struct Cli {
    /// Append debug logs and a perf summary on exit to this file
    /// (level via LAZYKEIFU_LOG, default "debug")
    #[arg(long, value_name = "PATH")]
    log_file: Option<PathBuf>,

    /// Listen for debug commands (NDJSON over TCP, e.g. 127.0.0.1:7167)
    #[arg(long, value_name = "ADDR")]
    debug_listen: Option<String>,

    /// Where file diffs are drawn, overriding the config file
    #[arg(long, value_enum)]
    file_view: Option<FileView>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if let Some(path) = &cli.log_file {
        logging::init(path)?;
    }
    let debug_rx = match &cli.debug_listen {
        Some(addr) => Some(debug_server::spawn(addr)?),
        None => None,
    };

    // Restore the terminal on panic
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = tui::restore();
        original_hook(panic_info);
    }));

    configure_git_extensions()?;

    // Load syntax highlighting in the background so the first file open is fast
    std::thread::spawn(ui::file_diff_view::prewarm);

    // Initialize application
    let mut app = App::new()?;
    if let Some(file_view) = cli.file_view {
        app.set_file_view(file_view);
    }

    // Initialize terminal
    let mut terminal = tui::init()?;
    let mut event_reader = match EventReader::new() {
        Ok(reader) => reader,
        Err(error) => {
            let _ = tui::restore();
            return Err(error);
        }
    };
    let mut last_user_input = Instant::now();

    // Main loop
    loop {
        // Render
        let draw_started = std::time::Instant::now();
        terminal.draw(|frame| {
            ui::draw(frame, &mut app);
        })?;
        app.perf.record("draw", draw_started.elapsed());

        // Exit check
        if app.should_quit {
            break;
        }

        // Process all queued events before the next render
        let poll_timeout = if app.has_pending_diff() {
            event::BUSY_POLL_TIMEOUT
        } else {
            event::IDLE_POLL_TIMEOUT
        };
        let batch = event_reader.poll_events(poll_timeout)?;
        if batch.had_input() {
            last_user_input = Instant::now();
        }
        let raw_count = batch.raw_count();
        let retained_count = batch.retained_count();
        let events = batch.into_events();
        if raw_count > 0 || retained_count > 0 {
            tracing::trace!(raw_count, retained_count, "input batch normalized");
        }
        if !events.is_empty() {
            let events_started = std::time::Instant::now();
            for event in events {
                match event {
                    InputEvent::Terminal(Event::Key(key)) => {
                        if let Some(action) = map_key_to_action(key, &app.mode) {
                            if let Err(e) = app.handle_action(action) {
                                // Show errors in the UI
                                app.show_error(format!("{}", e));
                            }
                        }
                    }
                    InputEvent::Terminal(Event::Mouse(mouse_event)) => {
                        mouse::handle_mouse(&mut app, mouse_event);
                    }
                    InputEvent::Scroll {
                        mouse: mouse_event,
                        steps,
                    } => mouse::handle_scroll(&mut app, mouse_event, steps),
                    // Resize events trigger redraw automatically
                    _ => {}
                }
                if app.should_quit {
                    break;
                }
            }
            app.perf.record("events", events_started.elapsed());
        }

        // Process pending debug commands
        if let Some(rx) = &debug_rx {
            while let Ok(command) = rx.try_recv() {
                let size = terminal.size()?;
                let response = debug_server::handle_request(
                    &mut app,
                    size.width,
                    size.height,
                    command.request,
                );
                let _ = command.reply.send(response);
            }
        }

        let allow_repository_refresh = last_user_input.elapsed() >= MAINTENANCE_IDLE_PERIOD;
        app.update_fetch_status(allow_repository_refresh);
        app.update_push_status(allow_repository_refresh);
        // Repository refreshes are synchronous, so they must not compete with
        // an active input gesture for the UI thread.
        if allow_repository_refresh {
            app.check_auto_refresh();
        }
    }

    app.perf.log_summary();

    // Restore terminal
    tui::restore()?;

    Ok(())
}
