use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::frontmatter;

#[derive(Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub tags: Vec<String>,
    pub git_branch: Option<String>,
}

pub const TODO_TAG: &str = "todo";

impl Config {
    pub fn branch(&self) -> String {
        self.git_branch.clone().unwrap_or("main".into())
    }

    pub fn load(dir: &Path) -> Config {
        fs::read_to_string(dir.join("config.json"))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }
}

pub struct Note {
    pub path: PathBuf,
    pub created: String,
    pub priority: Option<String>,
    pub tags: Vec<String>,
    pub title: String,
    pub body: String,
}

impl Note {
    pub fn search_text(&self) -> String {
        let prio = self.priority.as_ref().map(|p| format!(" [{p}]")).unwrap_or_default();
        let tags: Vec<String> = self.tags.iter().map(|t| format!("#{t}")).collect();
        format!("[{}]{prio} {} {}", self.created, tags.join(" "), self.title)
    }

    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t.eq_ignore_ascii_case(tag))
    }
}

pub fn parse_note(path: PathBuf, content: &str) -> Option<Note> {
    let (fields, body) = frontmatter::parse(content)?;
    let title = body.lines().find(|l| l.starts_with('#')).map(|l| l.trim_start_matches('#').trim());
    Some(Note {
        path,
        created: fields.created,
        priority: fields.priority,
        tags: fields.tags,
        title: title.unwrap_or_default().to_string(),
        body: body.to_string(),
    })
}

pub fn load_all(data_dir: &Path) -> Vec<Note> {
    let mut notes: Vec<Note> = fs::read_dir(data_dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "md"))
        .filter_map(|p| {
            let content = fs::read_to_string(&p).ok()?;
            parse_note(p, &content)
        })
        .collect();
    notes.sort_by(|a, b| b.created.cmp(&a.created).then_with(|| a.path.cmp(&b.path)));
    notes
}

#[derive(Debug, PartialEq)]
pub struct Query {
    pub include: Vec<String>,
    pub exclude: Vec<String>,
    pub text: String,
}

pub fn parse_query(q: &str) -> Query {
    let mut query = Query { include: vec![], exclude: vec![], text: String::new() };
    let mut text = vec![];
    for word in q.split_whitespace() {
        match (word.strip_prefix('+'), word.strip_prefix('-')) {
            (Some(t), _) if !t.is_empty() => query.include.push(t.to_string()),
            (_, Some(t)) if !t.is_empty() => query.exclude.push(t.to_string()),
            _ => text.push(word),
        }
    }
    query.text = text.join(" ");
    query
}

impl Query {
    pub fn matches_tag_filters(&self, note: &Note) -> bool {
        let like = |q: &String| note.tags.iter().any(|t| t.to_lowercase().contains(&q.to_lowercase()));
        self.include.iter().all(like) && !self.exclude.iter().any(like)
    }
}

pub fn today() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

fn new_path(data_dir: &Path) -> PathBuf {
    let id = uuid::Uuid::new_v4().simple().to_string();
    data_dir.join(format!("{}.md", &id[..8]))
}

pub fn new_note(data_dir: &Path, tags: &[String]) -> io::Result<PathBuf> {
    let mut content = format!("---\ncreated: {}\n", today());
    if tags.iter().any(|t| t.eq_ignore_ascii_case(TODO_TAG)) {
        content.push_str("priority: medium\n");
    }
    content.push_str("tags:\n");
    for tag in tags {
        content.push_str(&format!("  - {tag}\n"));
    }
    content.push_str("---\n\n#\n");
    let path = new_path(data_dir);
    fs::write(&path, content)?;
    Ok(path)
}

pub fn clone_note(path: &Path) -> io::Result<PathBuf> {
    let content = fs::read_to_string(path)?;
    let cloned = frontmatter::set_field(&content, "created", &today());
    let new = new_path(path.parent().unwrap_or(Path::new(".")));
    fs::write(&new, cloned)?;
    Ok(new)
}

pub fn mark_done(content: &str) -> String {
    frontmatter::replace_tag(content, TODO_TAG, "done")
}

pub struct JournalEntry {
    pub title: String,
    pub line: usize,
    pub body: String,
}

pub fn journal_entries(content: &str) -> Vec<JournalEntry> {
    let mut entries: Vec<JournalEntry> = vec![];
    for (i, line) in content.lines().enumerate() {
        if let Some(title) = line.strip_prefix("### ") {
            entries.push(JournalEntry { title: title.to_string(), line: i + 1, body: String::new() });
        }
        if let Some(e) = entries.last_mut() {
            e.body.push_str(line);
            e.body.push('\n');
        }
    }
    entries
}

pub fn with_today_heading(content: &str, today: &str) -> Option<String> {
    let first = content.lines().find_map(|l| l.strip_prefix("### "));
    if first.is_some_and(|t| t.starts_with(today)) {
        return None;
    }
    Some(format!("### {today}\n\n{content}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOTE: &str = "---\ncreated: 2026-07-30\npriority: medium\ntags:\n  - work\n  - todo\n---\n\n# TLID\n- todo stays\n";

    #[test]
    fn parses_frontmatter_and_title() {
        let n = parse_note(PathBuf::from("a.md"), NOTE).unwrap();
        assert_eq!(n.created, "2026-07-30");
        assert_eq!(n.priority.as_deref(), Some("medium"));
        assert_eq!(n.tags, ["work", "todo"]);
        assert_eq!(n.title, "TLID");
        assert_eq!(n.search_text(), "[2026-07-30] [medium] #work #todo TLID");
        assert!(parse_note(PathBuf::from("b.md"), "# no frontmatter").is_none());
    }

    #[test]
    fn query_splits_tags_and_text() {
        let q = parse_query("+work -done meet up");
        assert_eq!(q, Query { include: vec!["work".into()], exclude: vec!["done".into()], text: "meet up".into() });
        let n = parse_note(PathBuf::from("a.md"), NOTE).unwrap();
        assert!(parse_query("+WORK").matches_tag_filters(&n));
        assert!(parse_query("+wo").matches_tag_filters(&n));
        assert!(!parse_query("+meeting").matches_tag_filters(&n));
        assert!(!parse_query("-tod").matches_tag_filters(&n));
    }

    #[test]
    fn mark_done_only_touches_frontmatter() {
        let done = mark_done(NOTE);
        assert!(done.contains("  - done\n---"));
        assert!(done.contains("- todo stays\n"));
    }

    #[test]
    fn today_heading_added_once() {
        let j = "### 2026-08-25\n\nhello\n";
        let added = with_today_heading(j, "2026-10-19").unwrap();
        assert!(added.starts_with("### 2026-10-19\n\n### 2026-08-25"));
        assert!(with_today_heading(&added, "2026-10-19").is_none());
    }

    #[test]
    fn journal_splits_entries() {
        let j = "### 2026-08-25\na\n### 2025-12-10 - Wednesday\n#### Work\nb\n";
        let e = journal_entries(j);
        assert_eq!(e.len(), 2);
        assert_eq!((e[1].title.as_str(), e[1].line), ("2025-12-10 - Wednesday", 3));
        assert_eq!(e[1].body, "### 2025-12-10 - Wednesday\n#### Work\nb\n");
    }

    #[test]
    fn title_ignores_heading_level() {
        let n = parse_note(PathBuf::from("a.md"), "---\ncreated: x\n---\n\n## Sub title\n").unwrap();
        assert_eq!(n.title, "Sub title");
    }
}
