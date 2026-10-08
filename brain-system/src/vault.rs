//! Memory-vault scanning: notes, one shared classifier, and an Obsidian-style
//! wikilink extractor that ignores code and resolves case-insensitively.

use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::util;

#[derive(Clone, Debug)]
pub struct Note {
    pub rel: String,       // vault-relative, forward slashes
    pub path: PathBuf,
    pub stem: String,
    pub title: String,
    pub kind: &'static str, // session | entity | concept | inbox | skillnote | note
    pub folder: String,
    pub mtime: i64,
    pub size: u64,
    pub summary: String,
    pub links: Vec<String>, // raw link targets, deduped, order kept
}

/// The single classifier used by the wiki list, note detail and the brain.
pub fn classify(rel: &str) -> &'static str {
    let lower = rel.to_lowercase();
    let file = lower.rsplit('/').next().unwrap_or(&lower);
    if lower.starts_with("00-inbox/") { "inbox" }
    else if file.starts_with("session-") || lower.contains("/sessions/") { "session" }
    else if lower.split('/').any(|seg| seg == "skills") { "skillnote" }
    else if lower.starts_with("20-concepts/") { "concept" }
    else if lower.starts_with("10-entities/") { "entity" }
    else { "note" }
}

fn skip_entry(e: &walkdir::DirEntry) -> bool {
    let name = e.file_name().to_string_lossy();
    e.depth() > 0 && (name.starts_with('.') || name == "_system")
}

/// Scan the vault, reusing cached notes whose mtime+size didn't change.
pub fn scan(vault: &Path, cache: &HashMap<String, Note>) -> Vec<Note> {
    let mut out = Vec::new();
    for entry in WalkDir::new(vault).into_iter().filter_entry(|e| !skip_entry(e)).filter_map(|e| e.ok()) {
        let p = entry.path();
        if !entry.file_type().is_file() || p.extension().map_or(true, |x| x != "md") { continue; }
        let rel = match p.strip_prefix(vault) { Ok(r) => r.to_string_lossy().replace('\\', "/"), Err(_) => continue };
        let meta = match entry.metadata() { Ok(m) => m, Err(_) => continue };
        let mtime = meta.modified().ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as i64).unwrap_or(0);
        if let Some(c) = cache.get(&rel) {
            if c.mtime == mtime && c.size == meta.len() { out.push(c.clone()); continue; }
        }
        let content = fs::read_to_string(p).unwrap_or_default();
        out.push(build_note(vault, p, rel, mtime, meta.len(), &content));
    }
    out.sort_by(|a, b| a.rel.cmp(&b.rel));
    out
}

pub fn build_note(vault: &Path, p: &Path, rel: String, mtime: i64, size: u64, content: &str) -> Note {
    let stem = p.file_stem().unwrap_or_default().to_string_lossy().to_string();
    let folder = Path::new(&rel).parent().map(|x| x.to_string_lossy().replace('\\', "/")).unwrap_or_default();
    let (title, summary) = title_and_summary(content, &stem);
    let _ = vault;
    Note {
        kind: classify(&rel),
        rel, path: p.to_path_buf(), stem, title, folder, mtime, size, summary,
        links: extract_wikilinks(content),
    }
}

/// Skips YAML frontmatter; title = frontmatter `title:` or first `# ` heading.
pub fn title_and_summary(content: &str, stem: &str) -> (String, String) {
    let mut title = None;
    let mut body_lines = Vec::new();
    let mut in_fm = false;
    for (i, line) in content.lines().enumerate() {
        let t = line.trim();
        if i == 0 && t == "---" { in_fm = true; continue; }
        if in_fm {
            if t == "---" { in_fm = false; continue; }
            if let Some(v) = t.strip_prefix("title:") {
                let v = v.trim().trim_matches('"').trim_matches('\'');
                if !v.is_empty() { title = Some(v.to_string()); }
            }
            continue;
        }
        if title.is_none() {
            if let Some(h) = t.strip_prefix("# ") { title = Some(h.trim().to_string()); continue; }
        }
        if !t.is_empty() && !t.starts_with('#') && body_lines.len() < 4 { body_lines.push(t); }
    }
    (title.unwrap_or_else(|| stem.to_string()), util::one_line(&body_lines.join(" "), 240))
}

/// `[[target]]`, `[[target|alias]]`, `[[target#heading]]`, `![[embed]]` — but
/// never inside fenced code blocks or inline `code` spans (so C++'s
/// `[[nodiscard]]` or a prose example of `[[Wikilinks]]` isn't a link).
pub fn extract_wikilinks(content: &str) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    let mut fence: Option<&str> = None;
    for line in content.lines() {
        let t = line.trim_start();
        if let Some(f) = fence {
            if t.starts_with(f) { fence = None; }
            continue;
        }
        if t.starts_with("```") { fence = Some("```"); continue; }
        if t.starts_with("~~~") { fence = Some("~~~"); continue; }
        let stripped = strip_inline_code(line);
        let mut rest = stripped.as_str();
        while let Some(start) = rest.find("[[") {
            let after = &rest[start + 2..];
            let Some(end) = after.find("]]") else { break };
            let raw = &after[..end];
            if let Some(target) = clean_target(raw) {
                if seen.insert(target.to_lowercase()) { out.push(target); }
            }
            rest = &after[end + 2..];
        }
    }
    out
}

fn strip_inline_code(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut in_code = false;
    for c in line.chars() {
        if c == '`' { in_code = !in_code; continue; }
        if !in_code { out.push(c); }
    }
    out
}

fn clean_target(raw: &str) -> Option<String> {
    let t = raw.split('|').next().unwrap_or(raw);
    let t = t.split('#').next().unwrap_or(t);
    let t = t.split('^').next().unwrap_or(t).trim();
    let t = t.strip_suffix(".md").unwrap_or(t).trim();
    if t.is_empty() || t.contains('\n') || t.len() > 200 { return None; }
    Some(t.to_string())
}

/// Case-insensitive resolver: `name` or `folder/name`. Ambiguous stems resolve
/// to the shortest path, like Obsidian.
pub struct Resolver {
    by_stem: HashMap<String, Vec<usize>>,
    rels: Vec<String>,
}

impl Resolver {
    pub fn new(notes: &[Note]) -> Self {
        let mut by_stem: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, n) in notes.iter().enumerate() {
            by_stem.entry(n.stem.to_lowercase()).or_default().push(i);
        }
        for v in by_stem.values_mut() {
            v.sort_by_key(|&i| (notes[i].rel.len(), notes[i].rel.clone()));
        }
        Resolver { by_stem, rels: notes.iter().map(|n| n.rel.to_lowercase()).collect() }
    }

    pub fn resolve(&self, target: &str) -> Option<usize> {
        let lower = target.to_lowercase();
        let stem = lower.rsplit('/').next().unwrap_or(&lower);
        let cands = self.by_stem.get(stem)?;
        if lower.contains('/') {
            let suffix = format!("{}.md", lower);
            if let Some(&i) = cands.iter().find(|&&i| self.rels[i].ends_with(&suffix)) { return Some(i); }
        }
        cands.first().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn links_skip_code_and_strip_suffixes() {
        let md = "See [[Alpha]] and [[Beta|b]] and [[Gamma#Sec]].\n`[[nodiscard]]` here\n```\n[[InFence]]\n```\n[[alpha]] [[Delta.md]] ![[Img.png]]";
        assert_eq!(extract_wikilinks(md), vec!["Alpha", "Beta", "Gamma", "Delta", "Img.png"]);
    }

    #[test]
    fn classifier() {
        assert_eq!(classify("30-Logs/session-2026-09-01-x.md"), "session");
        assert_eq!(classify("10-Entities/Skills/pdf.md"), "skillnote");
        assert_eq!(classify("20-Concepts/Foo.md"), "concept");
        assert_eq!(classify("00-Inbox/x.md"), "inbox");
        assert_eq!(classify("10-Entities/Bar.md"), "entity");
    }

    #[test]
    fn title_from_frontmatter_and_heading() {
        let (t, s) = title_and_summary("---\ntitle: \"Hello\"\n---\n# Ignored\nbody line", "stem");
        assert_eq!(t, "Hello");
        assert_eq!(s, "body line");
        let (t2, _) = title_and_summary("# Head\ntext", "stem");
        assert_eq!(t2, "Head");
    }
}
