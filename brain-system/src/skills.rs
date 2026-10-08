//! The real skills library: every folder containing a SKILL.md is a skill.
//! Folders without SKILL.md that hold skills are categories. Skills can nest.

use std::path::Path;
use walkdir::WalkDir;

use crate::util;

#[derive(Clone, Debug, serde::Serialize)]
pub struct Skill {
    pub name: String,
    pub rel: String,              // library-relative folder, forward slashes
    pub skill_md: String,         // display path of SKILL.md
    pub category: Option<String>, // first non-skill ancestor folder
    pub parent: Option<String>,   // enclosing skill's rel, for nested skills
    pub description: String,
    pub mtime: i64,
}

pub fn scan(root: &Path) -> Vec<Skill> {
    let mut skills: Vec<Skill> = Vec::new();
    let walker = WalkDir::new(root).max_depth(6).into_iter().filter_entry(|e| {
        e.depth() == 0 || !e.file_name().to_string_lossy().starts_with('.')
    });
    for entry in walker.filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() || entry.file_name() != "SKILL.md" { continue; }
        let md = entry.path();
        let dir = match md.parent() { Some(d) => d, None => continue };
        let rel = match dir.strip_prefix(root) { Ok(r) => r.to_string_lossy().replace('\\', "/"), Err(_) => continue };
        if rel.is_empty() { continue; }
        let content = std::fs::read_to_string(md).unwrap_or_default();
        let (fm_name, desc) = frontmatter(&content);
        let dir_name = dir.file_name().unwrap_or_default().to_string_lossy().to_string();
        skills.push(Skill {
            name: fm_name.filter(|n| !n.is_empty()).unwrap_or(dir_name),
            skill_md: util::display_path(md),
            category: None,
            parent: None,
            description: util::one_line(&desc, 300),
            mtime: util::mtime_ms(md),
            rel,
        });
    }
    skills.sort_by(|a, b| a.rel.cmp(&b.rel));
    // Parent / category resolution needs the full set of skill folders.
    let rels: Vec<String> = skills.iter().map(|s| s.rel.clone()).collect();
    for s in skills.iter_mut() {
        let segs: Vec<&str> = s.rel.split('/').collect();
        let mut parent = None;
        for i in (1..segs.len()).rev() {
            let anc = segs[..i].join("/");
            if rels.contains(&anc) { parent = Some(anc); break; }
        }
        if parent.is_none() && segs.len() > 1 && !rels.contains(&segs[0].to_string()) {
            s.category = Some(segs[0].to_string());
        }
        s.parent = parent;
    }
    skills
}

fn frontmatter(content: &str) -> (Option<String>, String) {
    let mut lines = content.lines();
    if lines.next().map(|l| l.trim()) != Some("---") {
        return (None, first_paragraph(content));
    }
    let (mut name, mut desc) = (None, String::new());
    let mut block = false; // inside a YAML `description: |` / `>-` block
    for l in lines {
        let t = l.trim();
        if t == "---" { break; }
        if block {
            if l.starts_with(' ') || l.starts_with('\t') || t.is_empty() {
                if !t.is_empty() { if !desc.is_empty() { desc.push(' '); } desc.push_str(t); }
                continue;
            }
            block = false;
        }
        if let Some(v) = t.strip_prefix("name:") { name = Some(unquote(v)); }
        if let Some(v) = t.strip_prefix("description:") {
            let v = v.trim();
            if matches!(v, "|" | "|-" | "|+" | ">" | ">-" | ">+") { block = true; desc.clear(); }
            else { desc = unquote(v); }
        }
    }
    if desc.is_empty() { desc = first_paragraph(content); }
    (name, desc)
}

fn unquote(v: &str) -> String {
    v.trim().trim_matches('"').trim_matches('\'').to_string()
}

fn first_paragraph(content: &str) -> String {
    content.lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#') && !l.starts_with("---") && !l.starts_with("<!--"))
        .take(2).collect::<Vec<_>>().join(" ")
}

/// Map a skill identifier as agents write it ("pdf", "anthropic-skills:pdf",
/// "/gauntlet-loop") to the bare lowercase name used for matching.
pub fn bare_name(s: &str) -> String {
    let s = s.trim().trim_start_matches('/');
    s.rsplit(':').next().unwrap_or(s).trim().to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_scalar_description() {
        let (n, d) = frontmatter("---\nname: x\ndescription: >-\n  first line\n  second line\nother: 1\n---\nbody");
        assert_eq!(n.as_deref(), Some("x"));
        assert_eq!(d, "first line second line");
        let (_, d2) = frontmatter("---\ndescription: \"quoted\"\n---");
        assert_eq!(d2, "quoted");
    }
}
