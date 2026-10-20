#[derive(Default)]
pub struct Fields {
    pub created: String,
    pub priority: Option<String>,
    pub tags: Vec<String>,
}

fn is_fence(line: &str) -> bool {
    line.trim_end_matches(['\n', '\r']) == "---"
}

fn split(content: &str) -> Option<(&str, &str)> {
    let opening = content.split_inclusive('\n').next().filter(|l| is_fence(l))?;
    let mut end = opening.len();
    for line in content[end..].split_inclusive('\n') {
        end += line.len();
        if is_fence(line) {
            return Some(content.split_at(end));
        }
    }
    None
}

fn each_line<'a>(head: &'a str, mut f: impl FnMut(&'a str, Option<&'a str>)) {
    let mut in_tags = false;
    for line in head.split_inclusive('\n') {
        let tag = line.trim().strip_prefix("- ").filter(|_| in_tags);
        if tag.is_none() {
            in_tags = line.starts_with("tags:");
        }
        f(line, tag.map(str::trim));
    }
}

pub fn parse(content: &str) -> Option<(Fields, &str)> {
    let (head, body) = split(content)?;
    let mut fields = Fields::default();
    each_line(head, |line, tag| {
        if let Some(t) = tag {
            fields.tags.push(t.to_string());
        } else if let Some(v) = line.strip_prefix("created:") {
            fields.created = v.trim().to_string();
        } else if let Some(v) = line.strip_prefix("priority:") {
            fields.priority = Some(v.trim().to_string());
        }
    });
    Some((fields, body))
}

fn map(content: &str, mut f: impl FnMut(&str, Option<&str>) -> Option<String>) -> String {
    let Some((head, body)) = split(content) else {
        return content.to_string();
    };
    let mut out = String::new();
    each_line(head, |line, tag| {
        let bare = line.trim_end_matches(['\n', '\r']);
        match f(bare, tag) {
            Some(new) => {
                out.push_str(&new);
                out.push_str(&line[bare.len()..]);
            }
            None => out.push_str(line),
        }
    });
    out.push_str(body);
    out
}

pub fn set_field(content: &str, key: &str, value: &str) -> String {
    let prefix = format!("{key}:");
    map(content, |l, tag| (tag.is_none() && l.starts_with(&prefix)).then(|| format!("{prefix} {value}")))
}

pub fn replace_tag(content: &str, from: &str, to: &str) -> String {
    map(content, |l, tag| {
        tag.filter(|t| t.eq_ignore_ascii_case(from)).map(|_| {
            let indent = &l[..l.len() - l.trim_start().len()];
            format!("{indent}- {to}")
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_fields_and_body() {
        let (f, body) = parse("---\r\ncreated: x\r\npriority: high\r\ntags:\r\n  - work\r\n---\r\n# T\r\n").unwrap();
        assert_eq!((f.created.as_str(), f.priority.as_deref()), ("x", Some("high")));
        assert_eq!(f.tags, ["work"]);
        assert_eq!(body, "# T\r\n");
        assert_eq!(parse("---\ncreated: x\n---").unwrap().1, "");
        assert!(parse("# no frontmatter").is_none());
    }

    #[test]
    fn replace_tag_ignores_case_and_stays_in_tags() {
        let c = "---\naliases:\n  - todo\ntags:\n  - TODO\n---\n- todo\n";
        assert_eq!(replace_tag(c, "todo", "done"), "---\naliases:\n  - todo\ntags:\n  - done\n---\n- todo\n");
    }

    #[test]
    fn set_field_keeps_line_endings() {
        let c = "---\r\ncreated: x\r\n---\r\ncreated: body\r\n";
        assert_eq!(set_field(c, "created", "y"), "---\r\ncreated: y\r\n---\r\ncreated: body\r\n");
    }
}
