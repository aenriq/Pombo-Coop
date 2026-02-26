mod app;
mod config;
mod input;
mod provider;
mod theme;
mod ui;

use std::io::{self, Stdout};
use std::time::{Duration, Instant};

use app::App;
use crossterm::cursor::SetCursorStyle;
use crossterm::event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use input::{handle_key_event, handle_mouse_event};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;

fn main() -> io::Result<()> {
    let mut terminal = setup_terminal()?;
    let app_result = run_app(&mut terminal);
    restore_terminal(&mut terminal)?;
    app_result
}

fn setup_terminal() -> io::Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        EnableMouseCapture,
        SetCursorStyle::SteadyBlock
    )?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> io::Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture,
        SetCursorStyle::DefaultUserShape
    )?;
    terminal.show_cursor()?;
    Ok(())
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> io::Result<()> {
    let mut app = App::new();
    let mut needs_redraw = true;
    let mut show_caret = true;
    let mut last_blink = Instant::now();
    let mut last_thinking_tick = Instant::now();
    let blink_interval = Duration::from_millis(530);

    loop {
        if app.poll_background_updates() {
            show_caret = true;
            needs_redraw = true;
        }

        if needs_redraw {
            terminal.draw(|frame| ui::render(frame, &app, show_caret))?;
            needs_redraw = false;
            let _ = app.finalize_chat_scroll_anchor();
        }

        if app.should_quit() {
            return Ok(());
        }

        let mut timeout = Duration::from_millis(250);
        if app.composer_is_focused() {
            timeout = timeout.min(
                blink_interval
                    .checked_sub(last_blink.elapsed())
                    .unwrap_or(Duration::from_millis(0)),
            );
        }
        if app.thinking_animation_playing() {
            let thinking_interval = app.thinking_tick_interval();
            timeout = timeout.min(
                thinking_interval
                    .checked_sub(last_thinking_tick.elapsed())
                    .unwrap_or(Duration::from_millis(0)),
            );
        }

        if !event::poll(timeout)? {
            let mut should_redraw = false;
            if app.composer_is_focused() {
                if last_blink.elapsed() >= blink_interval {
                    show_caret = !show_caret;
                    last_blink = Instant::now();
                    should_redraw = true;
                }
            }
            if app.thinking_animation_playing() {
                let thinking_interval = app.thinking_tick_interval();
                if last_thinking_tick.elapsed() >= thinking_interval {
                    if app.advance_thinking_wave() {
                        should_redraw = true;
                    }
                    last_thinking_tick = Instant::now();
                }
            }
            if should_redraw {
                needs_redraw = true;
            }
            continue;
        }

        match event::read()? {
            Event::Key(key) => {
                if key.kind == KeyEventKind::Press {
                    handle_key_event(&mut app, key);
                    show_caret = true;
                    last_blink = Instant::now();
                    last_thinking_tick = Instant::now();
                    needs_redraw = true;
                }
            }
            Event::Resize(_, _) => {
                show_caret = true;
                last_thinking_tick = Instant::now();
                needs_redraw = true;
            }
            Event::Mouse(mouse) => {
                let size = terminal.size()?;
                let area = Rect::new(0, 0, size.width, size.height);
                if handle_mouse_event(&mut app, mouse, area) {
                    show_caret = true;
                    last_thinking_tick = Instant::now();
                    needs_redraw = true;
                }
            }
            _ => {}
        }
    }
}
