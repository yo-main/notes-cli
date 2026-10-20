mod frontmatter;
mod notes;
mod sync;
mod theme;

use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::LazyLock;
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;
use std::{env, fs, thread};

use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Matcher, Utf32Str};
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::crossterm::{execute, terminal};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, HighlightSpacing, Padding, Paragraph, Row, Table, TableState, Wrap};

use notes::{Config, JournalEntry, Note, TODO_TAG};
use theme::{Dracula, HIGHLIGHT_BG};

const END_OF_FILE_LINE: usize = 100;
const JOURNAL_BODY_LINE: usize = 2;
const ROW_HEIGHT: u16 = 2;
const PROMPT: &str = "notes> ";

#[derive(Clone, Copy, PartialEq)]
enum View {
    All,
    Todo,
    Journal,
}

impl View {
    fn next(self) -> View {
        match self {
            View::All => View::Todo,
            View::Todo => View::Journal,
            View::Journal => View::All,
        }
    }

    fn prev(self) -> View {
        match self {
            View::All => View::Journal,
            View::Todo => View::All,
            View::Journal => View::Todo,
        }
    }
}

struct App {
    dir: PathBuf,
    config: Config,
    view: View,
    query: String,
    cursor: usize,
    notes: Vec<Note>,
    journal: Vec<JournalEntry>,
    visible: Vec<usize>,
    table: TableState,
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
                .filter(|(_, n)| q.matches_tag_filters(n) && (view != View::Todo || n.has_tag(TODO_TAG)))
                .map(|(i, n)| (i, n.search_text()))
                .collect(),
        };
        let pattern = Pattern::parse(&q.text, CaseMatching::Ignore, Normalization::Smart);
        let mut matcher = Matcher::new(nucleo_matcher::Config::DEFAULT);
        let mut buf = vec![];
        let newest_first = labels
            .iter()
            .filter(|(_, l)| pattern.score(Utf32Str::new(l, &mut buf), &mut matcher).is_some())
            .map(|(i, _)| *i);
        self.visible = newest_first.rev().collect();
        let max = self.visible.len().saturating_sub(1);
        self.table.select(Some(self.table.selected().unwrap_or(max).min(max)));
    }

    fn reset_filter(&mut self) {
        self.table.select(None);
        self.refilter();
    }

    fn selected_index(&self) -> Option<usize> {
        self.table.selected().and_then(|i| self.visible.get(i).copied())
    }

    fn selected_note(&self) -> Option<&Note> {
        if self.view == View::Journal {
            return None;
        }
        self.selected_index().map(|i| &self.notes[i])
    }

    fn tags_for_new_note(&self) -> Vec<String> {
        let mut tags = notes::parse_query(&self.query).include;
        if self.view == View::Todo && !tags.iter().any(|t| t.eq_ignore_ascii_case(TODO_TAG)) {
            tags.push(TODO_TAG.into());
        }
        if tags.is_empty() { self.config.tags.clone() } else { tags }
    }
}

fn prev_char(s: &str, i: usize) -> usize {
    s[..i].char_indices().next_back().map_or(0, |(j, _)| j)
}

fn next_char(s: &str, i: usize) -> usize {
    s[i..].chars().next().map_or(i, |c| i + c.len_utf8())
}

fn word_left(s: &str, i: usize) -> usize {
    s[..i].trim_end_matches(' ').rfind(' ').map_or(0, |j| j + 1)
}

fn word_right(s: &str, i: usize) -> usize {
    let rest = &s[i..];
    let skip = rest.len() - rest.trim_start_matches(' ').len();
    rest[skip..].find(' ').map_or(s.len(), |j| i + skip + j)
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

fn open_selected(app: &mut App, terminal: &mut DefaultTerminal) -> io::Result<()> {
    if app.view == View::Journal {
        if let Some(i) = app.selected_index() {
            edit(terminal, &app.journal_path(), Some(app.journal[i].line))?;
        }
    } else if let Some(n) = app.selected_note() {
        edit(terminal, &n.path, None)?;
    }
    app.reload();
    Ok(())
}

fn create_entry(app: &mut App, terminal: &mut DefaultTerminal) -> io::Result<()> {
    if app.view == View::Journal {
        let path = app.journal_path();
        let content = fs::read_to_string(&path).unwrap_or_default();
        if let Some(new) = notes::with_today_heading(&content, &notes::today()) {
            fs::write(&path, new)?;
        }
        edit(terminal, &path, Some(JOURNAL_BODY_LINE))?;
    } else {
        let path = notes::new_note(&app.data_dir(), &app.tags_for_new_note())?;
        edit(terminal, &path, Some(END_OF_FILE_LINE))?;
    }
    app.reload();
    Ok(())
}

fn clone_selected(app: &mut App, terminal: &mut DefaultTerminal) -> io::Result<()> {
    if let Some(n) = app.selected_note() {
        let path = notes::clone_note(&n.path)?;
        edit(terminal, &path, Some(END_OF_FILE_LINE))?;
        app.reload();
    }
    Ok(())
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
            app.view = app.view.next();
            app.reset_filter();
        }
        KeyCode::BackTab => {
            app.view = app.view.prev();
            app.reset_filter();
        }
        KeyCode::Up => app.table.select_previous(),
        KeyCode::Char('k') if ctrl => app.table.select_previous(),
        KeyCode::Down => app.table.select_next(),
        KeyCode::Char('j') if ctrl => app.table.select_next(),
        KeyCode::Enter => open_selected(app, terminal)?,
        KeyCode::Char('n') if ctrl => create_entry(app, terminal)?,
        KeyCode::Char('b') if ctrl => clone_selected(app, terminal)?,
        KeyCode::Char('d') if ctrl => app.confirm_delete = app.selected_note().is_some(),
        KeyCode::Char(' ') if ctrl => {
            if let Some(n) = app.selected_note() {
                fs::write(&n.path, notes::mark_done(&fs::read_to_string(&n.path)?))?;
                app.reload();
            }
        }
        KeyCode::Backspace if app.cursor > 0 => {
            app.cursor = prev_char(&app.query, app.cursor);
            app.query.remove(app.cursor);
            app.reset_filter();
        }
        KeyCode::Char(c) if !ctrl => {
            app.query.insert(app.cursor, c);
            app.cursor += c.len_utf8();
            app.reset_filter();
        }
        KeyCode::Left if ctrl => app.cursor = word_left(&app.query, app.cursor),
        KeyCode::Right if ctrl => app.cursor = word_right(&app.query, app.cursor),
        KeyCode::Left => app.cursor = prev_char(&app.query, app.cursor),
        KeyCode::Right => app.cursor = next_char(&app.query, app.cursor),
        KeyCode::Char('a') if ctrl => app.cursor = 0,
        KeyCode::Char('e') if ctrl => app.cursor = app.query.len(),
        _ => {}
    }
    Ok(false)
}

static MARKDOWN: LazyLock<tui_markdown::Options<Dracula>> = LazyLock::new(|| {
    let theme = tui_markdown::CodeTheme::from_textmate(include_str!("../assets/Dracula.tmTheme"))
        .expect("embedded Dracula theme is valid");
    tui_markdown::Options::new(Dracula).code_theme(theme)
});

fn table_cells(n: &Note) -> Vec<String> {
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
        .visible
        .iter()
        .map(|&i| if app.view == View::Journal { vec![app.journal[i].title.clone()] } else { table_cells(&app.notes[i]) })
        .collect();
    let ncols = rows.first().map_or(1, Vec::len);
    let widths: Vec<Constraint> = (0..ncols)
        .map(|c| match c + 1 == ncols {
            true => Constraint::Fill(1),
            false => Constraint::Length(rows.iter().map(|r| r[c].chars().count()).max().unwrap_or(0) as u16),
        })
        .collect();
    let height = (rows.len() as u16 * ROW_HEIGHT).min(list.height);
    let list = Rect { y: list.bottom() - height, height, ..list };
    let table = Table::new(rows.into_iter().map(|r| Row::new(r).bottom_margin(ROW_HEIGHT - 1)), widths)
        .column_spacing(3)
        .row_highlight_style(Style::new().bg(HIGHLIGHT_BG).bold())
        .highlight_symbol(Span::from("▌ ").red())
        .highlight_spacing(HighlightSpacing::Always);
    frame.render_stateful_widget(table, list, &mut app.table);

    let help = match app.confirm_delete {
        true => Line::from("Delete this note? y/n").red().bold(),
        false => {
            let done = if app.view == View::Todo { "ctrl+space: done · " } else { "" };
            Line::from(format!("enter: open · ctrl+n: new · ctrl+b: clone · ctrl+d: delete · {done}tab/shift+tab: view")).dim()
        }
    };
    frame.render_widget(Paragraph::new(help), header);

    let total = match app.view {
        View::All => app.notes.len(),
        View::Todo => app.notes.iter().filter(|n| n.has_tag(TODO_TAG)).count(),
        View::Journal => app.journal.len(),
    };
    let mut line = Line::from(Span::from(format!("  {}/{total} · ", app.visible.len())).dim());
    for (view, label) in [(View::All, "all"), (View::Todo, "todo"), (View::Journal, "journal")] {
        let span = Span::from(format!(" {label} "));
        line.push_span(if view == app.view { span.reversed().bold() } else { span.dim() });
    }
    line.push_span(Span::from(format!(" · {} ", app.status)).dim());
    let rule = "─".repeat((info.width as usize).saturating_sub(line.width()));
    line.push_span(Span::from(rule).dim());
    frame.render_widget(Paragraph::new(line), info);

    let line = Line::from(vec![Span::from(PROMPT).blue().bold(), Span::from(app.query.as_str())]);
    let before = Span::from(&app.query[..app.cursor]).width() + PROMPT.len();
    frame.set_cursor_position((prompt.x + before as u16, prompt.y));
    frame.render_widget(Paragraph::new(line), prompt);

    let body = match (app.view, app.selected_index()) {
        (View::Journal, Some(i)) => app.journal[i].body.as_str(),
        (_, Some(i)) => app.notes[i].body.as_str(),
        _ => "",
    };
    let text = tui_markdown::from_str_with_options(body, &MARKDOWN);
    let block = Block::new().borders(Borders::TOP).border_style(Style::new().dim()).padding(Padding::new(2, 2, 1, 0));
    frame.render_widget(Paragraph::new(text).block(block).wrap(Wrap { trim: false }), preview);
}

fn initial_query(args: impl Iterator<Item = String>) -> String {
    let filters: Vec<String> = args.map(|a| if a.starts_with(['+', '-']) { a } else { format!("+{a}") }).collect();
    if filters.is_empty() { String::new() } else { filters.join(" ") + " " }
}

fn main() -> io::Result<()> {
    let dir = env::var("NOTES_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(env::var("HOME").unwrap_or_default()).join(".config/notes"));
    let config = Config::load(&dir);
    let query = initial_query(env::args().skip(1));

    let (tx, rx) = mpsc::channel();
    let (sync_dir, branch) = (dir.clone(), config.branch());
    thread::spawn(move || tx.send(sync::sync_blocking(&sync_dir, &branch)));

    let mut app = App {
        dir,
        config,
        view: View::All,
        cursor: query.len(),
        query,
        notes: vec![],
        journal: vec![],
        visible: vec![],
        table: TableState::default(),
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
    sync::spawn_detached(&app.dir, &app.config.branch());
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_moves() {
        let s = "+work  é meet";
        assert_eq!(word_left(s, s.len()), 10);
        assert_eq!(word_left(s, 10), 7);
        assert_eq!(word_left(s, 7), 0);
        assert_eq!(word_right(s, 0), 5);
        assert_eq!(word_right(s, 5), 9);
        assert_eq!(word_right(s, 9), s.len());
        assert_eq!(next_char(s, 7), 9);
        assert_eq!(prev_char(s, 9), 7);
        assert_eq!(prev_char(s, 0), 0);
    }
}
