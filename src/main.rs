use std::io::{self, Stdout};
use std::time::Duration;

use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind,
};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::{Frame, Terminal};

#[derive(Clone)]
struct FileChange {
    path: &'static str,
    additions: u16,
    deletions: u16,
}

#[derive(Clone)]
struct Worktree {
    repo: &'static str,
    name: &'static str,
    branch: &'static str,
    status: &'static str,
    pr_number: u16,
    summary: &'static str,
    changed_files: Vec<FileChange>,
}

struct App {
    worktrees: Vec<Worktree>,
    selected_idx: usize,
    should_quit: bool,
}

impl App {
    fn new() -> Self {
        Self {
            worktrees: vec![
                Worktree {
                    repo: "conductor",
                    name: "Planner",
                    branch: "epic-b-shell",
                    status: "In progress",
                    pr_number: 1,
                    summary: "Planning panel state + routing behavior.",
                    changed_files: vec![
                        FileChange {
                            path: "src/shell/planner.rs",
                            additions: 24,
                            deletions: 9,
                        },
                        FileChange {
                            path: "src/shell/events.rs",
                            additions: 6,
                            deletions: 2,
                        },
                    ],
                },
                Worktree {
                    repo: "conductor",
                    name: "Reviewer",
                    branch: "epic-b-diff",
                    status: "Merge conflicts",
                    pr_number: 2,
                    summary: "Diff parsing and conflict summarization changes.",
                    changed_files: vec![
                        FileChange {
                            path: "src/diff/parser.rs",
                            additions: 98,
                            deletions: 12,
                        },
                        FileChange {
                            path: "src/diff/ui.rs",
                            additions: 53,
                            deletions: 2,
                        },
                        FileChange {
                            path: "src/shell/right_panel.rs",
                            additions: 32,
                            deletions: 0,
                        },
                    ],
                },
                Worktree {
                    repo: "melty_home",
                    name: "Regression Sweep",
                    branch: "epic-b-regression",
                    status: "Needs changes",
                    pr_number: 4,
                    summary: "Catches keyboard edge cases in the composer.",
                    changed_files: vec![
                        FileChange {
                            path: "src/shell/textarea.rs",
                            additions: 0,
                            deletions: 73,
                        },
                        FileChange {
                            path: "src/shell/diff_panel.rs",
                            additions: 8,
                            deletions: 4,
                        },
                    ],
                },
            ],
            selected_idx: 0,
            should_quit: false,
        }
    }

    fn selected_worktree(&self) -> &Worktree {
        &self.worktrees[self.selected_idx]
    }

    fn next(&mut self) {
        if self.worktrees.is_empty() {
            self.selected_idx = 0;
            return;
        }
        self.selected_idx = (self.selected_idx + 1) % self.worktrees.len();
    }

    fn previous(&mut self) {
        if self.worktrees.is_empty() {
            self.selected_idx = 0;
            return;
        }
        self.selected_idx = if self.selected_idx == 0 {
            self.worktrees.len() - 1
        } else {
            self.selected_idx - 1
        };
    }
}

fn main() -> io::Result<()> {
    let mut terminal = setup_terminal()?;
    let app_result = run_app(&mut terminal);
    restore_terminal(&mut terminal)?;
    app_result
}

fn setup_terminal() -> io::Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> io::Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> io::Result<()> {
    let mut app = App::new();

    loop {
        terminal.draw(|frame| render(frame, &app))?;

        if app.should_quit {
            return Ok(());
        }

        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    handle_key_event(&mut app, key);
                }
            }
        }
    }
}

fn handle_key_event(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Down | KeyCode::Char('j') => app.next(),
        KeyCode::Up | KeyCode::Char('k') => app.previous(),
        _ => {}
    }
}

fn render(frame: &mut Frame, app: &App) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(34),
            Constraint::Percentage(33),
            Constraint::Percentage(33),
        ])
        .split(frame.area());

    render_left_panel(frame, app, columns[0]);
    render_middle_panel(frame, app, columns[1]);
    render_right_panel(frame, app, columns[2]);
}

fn render_left_panel(frame: &mut Frame, app: &App, area: Rect) {
    let list_items = app
        .worktrees
        .iter()
        .map(|worktree| {
            ListItem::new(vec![
                Line::from(vec![
                    Span::styled(worktree.repo, Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(" / "),
                    Span::raw(worktree.name),
                ]),
                Line::from(format!(
                    "{} · {} · #{}",
                    worktree.branch, worktree.status, worktree.pr_number
                )),
                Line::from(""),
            ])
        })
        .collect::<Vec<_>>();

    let list = List::new(list_items)
        .block(Block::default().title("Worktrees").borders(Borders::ALL))
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    let mut state = ListState::default();
    state.select(Some(app.selected_idx));
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_middle_panel(frame: &mut Frame, app: &App, area: Rect) {
    let selected = app.selected_worktree();
    let content = vec![
        Line::from(vec![
            Span::styled("Worktree: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(selected.name),
        ]),
        Line::from(vec![
            Span::styled("Repo: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(selected.repo),
        ]),
        Line::from(vec![
            Span::styled("Branch: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(selected.branch),
        ]),
        Line::from(vec![
            Span::styled("Status: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(selected.status),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Summary",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from(selected.summary),
        Line::from(""),
        Line::from("Keys: j/k or up/down to switch worktree, q to quit"),
    ];

    let panel = Paragraph::new(content)
        .block(Block::default().title("Details").borders(Borders::ALL))
        .wrap(Wrap { trim: false });
    frame.render_widget(panel, area);
}

fn render_right_panel(frame: &mut Frame, app: &App, area: Rect) {
    let selected = app.selected_worktree();
    let items = selected
        .changed_files
        .iter()
        .map(|change| {
            ListItem::new(Line::from(vec![
                Span::raw(change.path),
                Span::raw("  "),
                Span::styled(
                    format!("+{}", change.additions),
                    Style::default().fg(Color::Green),
                ),
                Span::raw(" "),
                Span::styled(
                    format!("-{}", change.deletions),
                    Style::default().fg(Color::Red),
                ),
            ]))
        })
        .collect::<Vec<_>>();

    let right_panel = List::new(items)
        .block(Block::default().title("Changed Files").borders(Borders::ALL));
    frame.render_widget(right_panel, area);
}
