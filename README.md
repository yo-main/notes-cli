# ratanote

A terminal UI for browsing, searching and editing markdown notes, with a journal, synced through [jj](https://github.com/jj-vcs/jj). Notes are edited in your editor; ratanote lists, filters, previews, creates, clones and deletes them.

## Data layout

Everything lives in `$NOTES_DIR` (default `~/.config/notes`):

- `data/*.md`: one note per file, starting with a frontmatter block, then a markdown body whose first `#` heading is the title.

  ```
  ---
  created: 2026-07-30
  priority: medium
  tags:
    - work
    - todo
  ---

  # Title
  ```

  Files without frontmatter are ignored.
- `journal.md`: one entry per `### ` heading.
- `config.json` (optional): `{"tags": ["inbox"], "git_branch": "main"}`. `tags` are the default tags of a new note, `git_branch` is the sync branch (default `main`).

## Environment

- `NOTES_DIR`: notes directory, default `~/.config/notes`.
- `EDITOR`: editor command, default `hx`. It is called as `$EDITOR path:line`, which is Helix syntax.

## Sync

If `$NOTES_DIR` is a jj repository with changes, ratanote runs a sync at start (in the background, status shown in the info line) and again on exit (detached, so it outlives the app). A sync does `jj git fetch`, rebases onto `<branch>@origin`, commits as `<date> - to describe`, moves the bookmark and pushes it. It does nothing outside a jj repo or when there are no changes. On exit, an unfinished startup sync is awaited first.

## Search

The prompt takes space-separated terms:

- `+tag`: keep notes with a tag containing `tag` (case-insensitive).
- `-tag`: drop notes with a tag containing `tag`.
- anything else: fuzzy match on the note line (`[created] [priority] #tags title`).

Command-line arguments become the initial filters: `ratanote work` starts with `+work`; arguments starting with `+` or `-` are used as given. The newest notes are shown at the bottom.

## Keys

| Key | Action |
| --- | --- |
| `tab` / `shift+tab` | Next / previous view (all, todo, journal) |
| `up` / `ctrl+k`, `down` / `ctrl+j` | Move selection |
| `enter` | Open the selected note, or the journal at the selected entry |
| `ctrl+n` | New note tagged from the `+` filters (todo view also adds `todo`; with no filters, the `config.json` tags); in the journal view, add a heading for today if the latest entry isn't today's, then open the journal |
| `ctrl+b` | Clone the selected note with today's date |
| `ctrl+d` | Delete the selected note, `y` to confirm, any other key cancels |
| `ctrl+space` | Mark the note's `todo` tag as `done` |
| `left` / `right`, `ctrl+left` / `ctrl+right` | Move the prompt cursor by character / word |
| `ctrl+a` / `ctrl+e` | Cursor to start / end of the prompt |
| `esc` / `ctrl+c` | Quit |
