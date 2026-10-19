mod notes;
mod sync;

use std::cmp::Reverse;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;
use std::{env, fs, thread};

use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Matcher, Utf32Str};
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::crossterm::{execute, terminal};
use ratatui::layout::{Constraint, Layout};
use ratatui::layout::Rect;
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, HighlightSpacing, Padding, Paragraph, Row, Table, TableState, Wrap};

use notes::{Config, Entry, Note};

#[derive(Clone, Copy, PartialEq)]
enum View {
    All,
    Todo,
    Journal,
}

struct App {
    dir: PathBuf,
    config: Config,
    view: View,
    query: String,
    notes: Vec<Note>,
    journal: Vec<Entry>,
    filtered: Vec<usize>,
    list: TableState,
    status: String,
    confirm_delete: bool,
    sync_rx: Option<Receiver<Result<(), String>>>,
}

impl App {
    fn data_dir(&self) -> PathBuf {
        self.dir.join("data")
    }

    fn journal_path(&self) -> PathBuf {
        self.dir.join("journal.md")
    }

    fn branch(&self) -> String {
        self.config.git_branch.clone().unwrap_or("main".into())
    }

    fn reload(&mut self) {
        self.notes = notes::load_all(&self.data_dir());
        self.journal = notes::journal_entries(&fs::read_to_string(self.journal_path()).unwrap_or_default());
        self.refilter();
    }

    fn refilter(&mut self) {
        let q = notes::parse_query(&self.query);
        let labels: Vec<(usize, String)> = match self.view {
            View::Journal => self.journal.iter().enumerate().map(|(i, e)| (i, e.title.clone())).collect(),
            view => self
                .notes
                .iter()
                .enumerate()
                .filter(|(_, n)| q.keeps(n) && (view != View::Todo || n.has_tag("todo")))
                .map(|(i, n)| (i, n.row()))
                .collect(),
        };
        let pattern = Pattern::parse(&q.text, CaseMatching::Ignore, Normalization::Smart);
        let mut matcher = Matcher::new(nucleo_matcher::Config::DEFAULT);
        let mut buf = vec![];
        let mut scored: Vec<(usize, u32)> = labels
            .iter()
            .filter_map(|(i, l)| pattern.score(Utf32Str::new(l, &mut buf), &mut matcher).map(|s| (*i, s)))
            .collect();
        scored.sort_by_key(|&(_, s)| Reverse(s));
        self.filtered = scored.into_iter().rev().map(|(i, _)| i).collect();
        let max = self.filtered.len().saturating_sub(1);
        self.list.select(Some(self.list.selected().unwrap_or(max).min(max)));
    }

    fn selected(&self) -> Option<usize> {
        self.list.selected().and_then(|i| self.filtered.get(i).copied())
    }

    fn selected_note(&self) -> Option<&Note> {
        if self.view == View::Journal {
            return None;
        }
        self.selected().map(|i| &self.notes[i])
    }

    fn new_tags(&self) -> Vec<String> {
        let mut tags = notes::parse_query(&self.query).include;
        if self.view == View::Todo && !tags.iter().any(|t| t.eq_ignore_ascii_case("todo")) {
            tags.push("todo".into());
        }
        if tags.is_empty() { self.config.tags.clone() } else { tags }
    }
}

fn edit(terminal: &mut DefaultTerminal, path: &Path, line: Option<usize>) -> io::Result<()> {
    ratatui::restore();
    let editor = env::var("EDITOR").unwrap_or("hx".into());
    // ponytail: `path:line` is Helix syntax, add `+line` if a vim-style $EDITOR is used
    let arg = match line {
        Some(l) => format!("{}:{l}", path.display()),
        None => path.display().to_string(),
    };
    let result = Command::new(editor).arg(arg).status();
    terminal::enable_raw_mode()?;
    execute!(io::stdout(), terminal::EnterAlternateScreen)?;
    terminal.clear()?;
    result.map(|_| ())
}

fn handle_key(app: &mut App, terminal: &mut DefaultTerminal, key: KeyEvent) -> io::Result<bool> {
    if app.confirm_delete {
        app.confirm_delete = false;
        if key.code == KeyCode::Char('y')
            && let Some(n) = app.selected_note()
        {
            fs::remove_file(&n.path)?;
            app.reload();
        }
        return Ok(false);
    }
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    match key.code {
        KeyCode::Esc => return Ok(true),
        KeyCode::Char('c') if ctrl => return Ok(true),
        KeyCode::Tab => {
            app.view = match app.view {
                View::All => View::Todo,
                View::Todo => View::Journal,
                View::Journal => View::All,
            };
            app.list.select(None);
            app.refilter();
        }
        KeyCode::BackTab => {
            app.view = match app.view {
                View::All => View::Journal,
                View::Todo => View::All,
                View::Journal => View::Todo,
            };
            app.list.select(None);
            app.refilter();
        }
        KeyCode::Up => app.list.select_previous(),
        KeyCode::Char('k') if ctrl => app.list.select_previous(),
        KeyCode::Down => app.list.select_next(),
        KeyCode::Char('j') if ctrl => app.list.select_next(),
        KeyCode::Enter => {
            if app.view == View::Journal {
                if let Some(i) = app.selected() {
                    edit(terminal, &app.journal_path(), Some(app.journal[i].line))?;
                }
            } else if let Some(n) = app.selected_note() {
                edit(terminal, &n.path.clone(), None)?;
            }
            app.reload();
        }
        KeyCode::Char('n') if ctrl => {
            if app.view == View::Journal {
                let path = app.journal_path();
                let content = fs::read_to_string(&path).unwrap_or_default();
                if let Some(new) = notes::with_today_heading(&content, &notes::today()) {
                    fs::write(&path, new)?;
                }
                edit(terminal, &path, Some(2))?;
            } else {
                let path = notes::new_note(&app.data_dir(), &app.new_tags())?;
                edit(terminal, &path, Some(100))?;
            }
            app.reload();
        }
        KeyCode::Char('b') if ctrl => {
            if let Some(n) = app.selected_note() {
                let path = notes::clone_note(&n.path)?;
                edit(terminal, &path, Some(100))?;
                app.reload();
            }
        }
        KeyCode::Char('d') if ctrl => app.confirm_delete = app.selected_note().is_some(),
        KeyCode::Char(' ') if ctrl => {
            if let Some(n) = app.selected_note() {
                fs::write(&n.path, notes::mark_done(&fs::read_to_string(&n.path)?))?;
                app.reload();
            }
        }
        KeyCode::Backspace => {
            app.query.pop();
            app.list.select(None);
            app.refilter();
        }
        KeyCode::Char(c) if !ctrl => {
            app.query.push(c);
            app.list.select(None);
            app.refilter();
        }
        _ => {}
    }
    Ok(false)
}

#[derive(Clone)]
struct Glow;

impl tui_markdown::StyleSheet for Glow {
    fn heading(&self, level: u8) -> Style {
        match level {
            1 => Style::new().fg(Color::Indexed(230)).bg(Color::Indexed(63)).bold(),
            _ => Style::new().fg(Color::Indexed(39)).bold(),
        }
    }

    fn heading_marker(&self, _level: u8) -> &str {
        ""
    }
}

fn cells(n: &Note) -> Vec<String> {
    let tag = |i: usize| n.tags.get(i).map(|t| format!("#{t}")).unwrap_or_default();
    let first = match &n.priority {
        Some(p) => format!("[{p}] {}", tag(0)),
        None => tag(0),
    };
    vec![format!("[{}]", n.created), first, tag(1), n.title.clone()]
}

fn draw(frame: &mut ratatui::Frame, app: &mut App) {
    let [list, header, info, prompt, preview] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Percentage(50),
    ])
    .areas(frame.area());

    let rows: Vec<Vec<String>> = app
        .filtered
        .iter()
        .map(|&i| if app.view == View::Journal { vec![app.journal[i].title.clone()] } else { cells(&app.notes[i]) })
        .collect();
    let ncols = rows.first().map_or(1, Vec::len);
    let widths: Vec<Constraint> = (0..ncols)
        .map(|c| match c + 1 == ncols {
            true => Constraint::Fill(1),
            false => Constraint::Length(rows.iter().map(|r| r[c].chars().count()).max().unwrap_or(0) as u16),
        })
        .collect();
    let height = (rows.len() as u16 * 2).min(list.height);
    let list = Rect { y: list.bottom() - height, height, ..list };
    let table = Table::new(rows.into_iter().map(|r| Row::new(r).bottom_margin(1)), widths)
        .column_spacing(3)
        .row_highlight_style(Style::new().bg(Color::Indexed(236)).bold())
        .highlight_symbol(Span::from("▌ ").red())
        .highlight_spacing(HighlightSpacing::Always);
    frame.render_stateful_widget(table, list, &mut app.list);

    let help = match app.confirm_delete {
        true => Line::from("Delete this note? y/n").red().bold(),
        false => Line::from("enter: open · ctrl+n: new · ctrl+b: clone · ctrl+d: delete · ctrl+space: done · tab/shift+tab: view").dim(),
    };
    frame.render_widget(Paragraph::new(help), header);

    let (view, total) = match app.view {
        View::All => ("all", app.notes.len()),
        View::Todo => ("todo", app.notes.iter().filter(|n| n.has_tag("todo")).count()),
        View::Journal => ("journal", app.journal.len()),
    };
    let counts = format!("  {}/{total} · {view} · {} ", app.filtered.len(), app.status);
    let rule = "─".repeat((info.width as usize).saturating_sub(counts.chars().count()));
    frame.render_widget(Paragraph::new(Line::from(vec![counts.into(), rule.into()]).dim()), info);

    let line = Line::from(vec![Span::from("notes> ").blue().bold(), Span::from(app.query.as_str())]);
    frame.set_cursor_position((prompt.x + line.width() as u16, prompt.y));
    frame.render_widget(Paragraph::new(line), prompt);

    let body = match (app.view, app.selected()) {
        (View::Journal, Some(i)) => app.journal[i].body.as_str(),
        (_, Some(i)) => app.notes[i].body.as_str(),
        _ => "",
    };
    let text = tui_markdown::from_str_with_options(body, &tui_markdown::Options::new(Glow));
    let block = Block::new().borders(Borders::TOP).border_style(Style::new().dim()).padding(Padding::new(2, 2, 1, 0));
    frame.render_widget(Paragraph::new(text).block(block).wrap(Wrap { trim: false }), preview);
}

fn main() -> io::Result<()> {
    let dir = env::var("NOTES_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(env::var("HOME").unwrap_or_default()).join(".config/notes"));
    let query: Vec<String> = env::args()
        .skip(1)
        .map(|a| if a.starts_with(['+', '-']) { a } else { format!("+{a}") })
        .collect();
    let config = Config::load(&dir);

    let (tx, rx) = mpsc::channel();
    let (sync_dir, branch) = (dir.clone(), config.git_branch.clone().unwrap_or("main".into()));
    thread::spawn(move || tx.send(sync::sync_blocking(&sync_dir, &branch)));

    let mut app = App {
        dir,
        config,
        view: View::All,
        query: if query.is_empty() { String::new() } else { query.join(" ") + " " },
        notes: vec![],
        journal: vec![],
        filtered: vec![],
        list: TableState::default(),
        status: "syncing…".into(),
        confirm_delete: false,
        sync_rx: Some(rx),
    };
    app.reload();

    let mut terminal = ratatui::init();
    let result = run(&mut terminal, &mut app);
    ratatui::restore();

    // a startup sync still in flight would race the detached one
    if let Some(rx) = app.sync_rx.take() {
        let _ = rx.recv();
    }
    sync::spawn_detached(&app.dir, &app.branch());
    result
}

fn run(terminal: &mut DefaultTerminal, app: &mut App) -> io::Result<()> {
    loop {
        if let Some(rx) = &app.sync_rx
            && let Ok(res) = rx.try_recv()
        {
            app.status = match res {
                Ok(()) => "synced".into(),
                Err(e) => format!("sync failed: {e}"),
            };
            app.sync_rx = None;
        }
        terminal.draw(|f| draw(f, app))?;
        if event::poll(Duration::from_millis(250))?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
            && handle_key(app, terminal, key)?
        {
            return Ok(());
        }
    }
}
