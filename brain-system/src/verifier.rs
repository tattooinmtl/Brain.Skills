//! The Verifier Librarian. Gold notes (10-Entities, 20-Concepts) are tracked
//! in a hash manifest with content snapshots. A sweep produces proposals for:
//!   * drift  — a Gold note changed since it was last verified
//!              approve = accept the new version; reject = restore the last
//!              verified version (the rejected text is archived, never lost)
//!   * link   — a broken [[wikilink]] with an obvious fix (case/spacing/typo)
//!              approve = rewrite the link in the note; reject = dismiss
//! Every resolved proposal is archived in `_system/archive/`, and archived ids
//! are never re-proposed.

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::{config, util, vault};

fn valid_proposal_id(s: &str) -> bool {
    !s.is_empty() && s.len() <= 64 && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct Proposal {
    pub proposal_id: String,
    pub created_at: String,
    pub entry_id: String, // vault-relative path of the Gold note
    pub title: String,
    pub current_text: String,
    #[serde(default)]
    pub citations: Vec<String>,
    pub reason: String,
    pub evidence_summary: String,
    pub suggested_edit: String,
    pub confidence: f64,
    pub status: String,
    #[serde(default)]
    pub kind: String, // drift | link | (legacy: empty = replace current_text with suggested_edit)
    #[serde(default)]
    pub base_hash: String,
    #[serde(default)]
    pub new_hash: String,
    #[serde(default)]
    pub resolved_at: String,
}

#[derive(serde::Serialize, Default, Clone)]
pub struct SweepReport {
    pub ran_at: String,
    pub checked: usize,
    pub baselined: usize,
    pub new_drift: usize,
    pub new_links: usize,
    pub pending: Vec<Proposal>,
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
struct Manifest {
    #[serde(default)]
    files: BTreeMap<String, ManifestEntry>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct ManifestEntry {
    hash: String,
    verified_at: String,
}

struct Paths { vault: PathBuf, inbox: PathBuf, archive: PathBuf, manifest: PathBuf, snapshots: PathBuf }

fn paths() -> Paths {
    let vault = config::vault_dir();
    let sys = vault.join("_system");
    Paths {
        inbox: vault.join("00-Inbox"),
        archive: sys.join("archive"),
        manifest: sys.join("verifier").join("manifest.json"),
        snapshots: sys.join("verifier").join("snapshots"),
        vault,
    }
}

fn is_gold(rel: &str) -> bool {
    let l = rel.to_lowercase();
    l.starts_with("10-entities/") || l.starts_with("20-concepts/")
}

fn load_manifest(p: &Paths) -> Manifest {
    fs::read_to_string(&p.manifest).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

fn save_manifest(p: &Paths, m: &Manifest) {
    let _ = util::atomic_write(&p.manifest, serde_json::to_string_pretty(m).unwrap_or_default().as_bytes());
}

fn snapshot(p: &Paths, hash: &str, content: &str) {
    let f = p.snapshots.join(format!("{}.md", hash));
    if !f.exists() { let _ = util::atomic_write(&f, content.as_bytes()); }
}

fn content_hash(content: &str) -> String {
    util::sha256_hex(content.as_bytes())
}

/// Proposals currently waiting in the inbox (no scanning).
pub fn pending() -> Vec<Proposal> {
    let p = paths();
    let mut out = Vec::new();
    if let Ok(rd) = fs::read_dir(&p.inbox) {
        for e in rd.flatten() {
            let path = e.path();
            if path.extension().map_or(false, |x| x == "json") {
                if let Some(prop) = fs::read_to_string(&path).ok().and_then(|t| serde_json::from_str::<Proposal>(&t).ok()) {
                    if prop.status == "pending" { out.push(prop); }
                }
            }
        }
    }
    out.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    out
}

pub fn scan_and_verify() -> SweepReport {
    let p = paths();
    let _ = fs::create_dir_all(&p.inbox);
    let notes = vault::scan(&p.vault, &Default::default());
    let resolver = vault::Resolver::new(&notes);
    let mut manifest = load_manifest(&p);
    let known: HashSet<String> = existing_ids(&p);
    let now = util::iso(util::now_ms());
    let mut report = SweepReport { ran_at: now.clone(), ..Default::default() };

    let stems: Vec<(String, String)> = notes.iter().map(|n| (squash(&n.stem), n.stem.clone())).collect();

    for n in notes.iter().filter(|n| is_gold(&n.rel)) {
        let Ok(content) = fs::read_to_string(&n.path) else { continue };
        report.checked += 1;
        let hash = content_hash(&content);
        match manifest.files.get(&n.rel).cloned() {
            None => {
                snapshot(&p, &hash, &content);
                manifest.files.insert(n.rel.clone(), ManifestEntry { hash, verified_at: now.clone() });
                report.baselined += 1;
            }
            Some(prev) if prev.hash != hash => {
                let id = format!("drift-{}", &util::sha256_hex(format!("{}{}", n.rel, hash).as_bytes())[..16]);
                if !known.contains(&id) {
                    let old = fs::read_to_string(p.snapshots.join(format!("{}.md", prev.hash))).unwrap_or_default();
                    let (removed, added) = line_diff(&old, &content);
                    snapshot(&p, &hash, &content);
                    write_proposal(&p, &Proposal {
                        proposal_id: id,
                        created_at: now.clone(),
                        entry_id: n.rel.clone(),
                        title: n.title.clone(),
                        current_text: if old.is_empty() { "(no snapshot of the verified version)".into() } else { removed.join("\n") },
                        citations: vec![],
                        reason: format!("Gold note changed since it was verified on {}.", prev.verified_at),
                        evidence_summary: format!("{} line(s) removed, {} line(s) added", removed.len(), added.len()),
                        suggested_edit: added.join("\n"),
                        confidence: 1.0,
                        status: "pending".into(),
                        kind: "drift".into(),
                        base_hash: prev.hash.clone(),
                        new_hash: hash,
                        resolved_at: String::new(),
                    });
                    report.new_drift += 1;
                }
            }
            _ => {}
        }

        for target in &n.links {
            if resolver.resolve(target).is_some() || target.contains('.') { continue; }
            let Some(fix) = close_match(target, &stems) else { continue };
            let id = format!("link-{}", &util::sha256_hex(format!("{}{}{}", n.rel, target, fix).as_bytes())[..16]);
            if known.contains(&id) { continue; }
            write_proposal(&p, &Proposal {
                proposal_id: id,
                created_at: now.clone(),
                entry_id: n.rel.clone(),
                title: n.title.clone(),
                current_text: format!("[[{}]]", target),
                citations: vec![],
                reason: format!("Broken link — no note named \"{}\". \"{}\" exists.", target, fix),
                evidence_summary: "Link target resolved by case/spacing/typo match".into(),
                suggested_edit: format!("[[{}]]", fix),
                confidence: 0.8,
                status: "pending".into(),
                kind: "link".into(),
                base_hash: String::new(),
                new_hash: String::new(),
                resolved_at: String::new(),
            });
            report.new_links += 1;
        }
    }
    // Forget deleted Gold notes.
    let live: HashSet<&str> = notes.iter().map(|n| n.rel.as_str()).collect();
    manifest.files.retain(|k, _| live.contains(k.as_str()));
    save_manifest(&p, &manifest);
    report.pending = pending();
    report
}

fn existing_ids(p: &Paths) -> HashSet<String> {
    let mut ids = HashSet::new();
    for dir in [&p.inbox, &p.archive] {
        if let Ok(rd) = fs::read_dir(dir) {
            for e in rd.flatten() {
                let name = e.file_name().to_string_lossy().to_string();
                if let Some(id) = name.strip_suffix(".json") { ids.insert(id.to_string()); }
            }
        }
    }
    ids
}

fn write_proposal(p: &Paths, prop: &Proposal) {
    let _ = util::atomic_write(&p.inbox.join(format!("{}.json", prop.proposal_id)), serde_json::to_string_pretty(prop).unwrap_or_default().as_bytes());
}

fn squash(s: &str) -> String {
    s.chars().filter(|c| c.is_alphanumeric()).collect::<String>().to_lowercase()
}

fn close_match(target: &str, stems: &[(String, String)]) -> Option<String> {
    let t = squash(target);
    if t.len() < 3 { return None; }
    if let Some((_, s)) = stems.iter().find(|(sq, _)| *sq == t) { return Some(s.clone()); }
    if t.len() < 6 { return None; }
    let mut best: Option<(usize, &String)> = None;
    for (sq, s) in stems {
        if sq.len().abs_diff(t.len()) > 2 { continue; }
        let d = levenshtein(&t, sq);
        if d <= 2 && best.map_or(true, |(bd, _)| d < bd) { best = Some((d, s)); }
    }
    best.map(|(_, s)| s.clone())
}

fn levenshtein(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut cur = vec![i + 1; b.len() + 1];
        for (j, cb) in b.iter().enumerate() {
            cur[j + 1] = (prev[j] + (ca != *cb) as usize).min(prev[j + 1] + 1).min(cur[j] + 1);
        }
        prev = cur;
    }
    prev[b.len()]
}

/// Lines only in `old` / only in `new` (multiset difference, order kept).
fn line_diff(old: &str, new: &str) -> (Vec<String>, Vec<String>) {
    let mut pool: BTreeMap<&str, i32> = BTreeMap::new();
    for l in new.lines() { *pool.entry(l).or_insert(0) += 1; }
    let mut removed = Vec::new();
    for l in old.lines() {
        match pool.get_mut(l) { Some(c) if *c > 0 => *c -= 1, _ => if !l.trim().is_empty() { removed.push(l.to_string()) } }
    }
    let mut pool_old: BTreeMap<&str, i32> = BTreeMap::new();
    for l in old.lines() { *pool_old.entry(l).or_insert(0) += 1; }
    let mut added = Vec::new();
    for l in new.lines() {
        match pool_old.get_mut(l) { Some(c) if *c > 0 => *c -= 1, _ => if !l.trim().is_empty() { added.push(l.to_string()) } }
    }
    (removed.into_iter().take(60).collect(), added.into_iter().take(60).collect())
}

/// A human edited this file through the UI: that edit is the verified
/// version, so it must not come back as a drift proposal.
pub fn mark_verified(path: &Path, content: &str) {
    let p = paths();
    let vault = p.vault.canonicalize().unwrap_or_else(|_| p.vault.clone());
    let Ok(rel) = path.strip_prefix(&vault) else { return };
    let rel = rel.to_string_lossy().replace('\\', "/");
    if !is_gold(&rel) { return; }
    let h = content_hash(content);
    snapshot(&p, &h, content);
    let mut m = load_manifest(&p);
    m.files.insert(rel, ManifestEntry { hash: h, verified_at: util::iso(util::now_ms()) });
    save_manifest(&p, &m);
}

/// Resolve a proposal. Errors carry an HTTP status for the API.
pub fn resolve_proposal(proposal_id: &str, action: &str) -> Result<String, (u16, String)> {
    if !valid_proposal_id(proposal_id) { return Err((400, "Invalid proposal id".into())); }
    if action != "approve" && action != "reject" {
        return Err((400, format!("Unknown action \"{}\" — expected approve or reject", action)));
    }
    let p = paths();
    let prop_file = p.inbox.join(format!("{}.json", proposal_id));
    let text = fs::read_to_string(&prop_file).map_err(|_| (404, "Proposal not found".to_string()))?;
    let mut prop: Proposal = serde_json::from_str(&text).map_err(|_| (422, "Proposal file is not valid JSON".to_string()))?;
    let target = note_path(&p.vault, &prop.entry_id).ok_or((404, format!("Target note {} not found", prop.entry_id)))?;
    let mut manifest = load_manifest(&p);
    let now = util::iso(util::now_ms());

    let message = match (prop.kind.as_str(), action) {
        ("drift", "approve") => {
            let content = fs::read_to_string(&target).map_err(|e| (500, e.to_string()))?;
            let h = content_hash(&content);
            if !prop.new_hash.is_empty() && h != prop.new_hash {
                return Err((409, "The note changed again after this proposal — run a new sweep.".into()));
            }
            snapshot(&p, &h, &content);
            manifest.files.insert(prop.entry_id.clone(), ManifestEntry { hash: h, verified_at: now.clone() });
            "New version accepted as verified Gold.".to_string()
        }
        ("drift", "reject") => {
            let snap = p.snapshots.join(format!("{}.md", prop.base_hash));
            let old = fs::read_to_string(&snap).map_err(|_| (409, "No snapshot of the verified version — cannot restore.".to_string()))?;
            let current = fs::read_to_string(&target).unwrap_or_default();
            let _ = util::atomic_write(&p.archive.join(format!("{}.rejected.md", proposal_id)), current.as_bytes());
            util::atomic_write(&target, old.as_bytes()).map_err(|e| (500, e.to_string()))?;
            manifest.files.insert(prop.entry_id.clone(), ManifestEntry { hash: prop.base_hash.clone(), verified_at: now.clone() });
            format!("Restored the verified version. Rejected text archived as {}.rejected.md.", proposal_id)
        }
        (_, "approve") => {
            // link fixes and legacy proposals: replace current_text with suggested_edit.
            let content = fs::read_to_string(&target).map_err(|e| (500, e.to_string()))?;
            if prop.current_text.is_empty() || !content.contains(&prop.current_text) {
                return Err((409, "The text this proposal edits is no longer in the note.".into()));
            }
            let updated = if prop.kind == "link" {
                replace_link(&content, &prop.current_text, &prop.suggested_edit)
            } else {
                content.replacen(&prop.current_text, &prop.suggested_edit, 1)
            };
            util::atomic_write(&target, updated.as_bytes()).map_err(|e| (500, e.to_string()))?;
            if is_gold(&prop.entry_id) {
                let h = content_hash(&updated);
                snapshot(&p, &h, &updated);
                manifest.files.insert(prop.entry_id.clone(), ManifestEntry { hash: h, verified_at: now.clone() });
            }
            format!("Wrote the edit into {}.", prop.entry_id)
        }
        _ => "Dismissed.".to_string(),
    };

    prop.status = if action == "approve" { "approved".into() } else { "rejected".into() };
    prop.resolved_at = now;
    util::atomic_write(&p.archive.join(format!("{}.json", proposal_id)), serde_json::to_string_pretty(&prop).unwrap_or_default().as_bytes())
        .map_err(|e| (500, format!("Could not archive proposal: {}", e)))?;
    let _ = fs::remove_file(&prop_file);
    save_manifest(&p, &manifest);
    Ok(message)
}

/// `[[old]]` -> `[[new]]`, also fixing `[[old|alias]]` and `[[old#heading]]`.
fn replace_link(content: &str, current: &str, suggested: &str) -> String {
    let old = current.trim_start_matches("[[").trim_end_matches("]]");
    let new = suggested.trim_start_matches("[[").trim_end_matches("]]");
    let mut s = content.to_string();
    for tail in ["]]", "|", "#"] {
        s = s.replace(&format!("[[{}{}", old, tail), &format!("[[{}{}", new, tail));
    }
    s
}

fn note_path(vault: &Path, rel: &str) -> Option<PathBuf> {
    if rel.is_empty() { return None; }
    let candidate = if Path::new(rel).is_absolute() { PathBuf::from(rel) } else { vault.join(rel) };
    util::sandboxed(&candidate.to_string_lossy(), &[vault.to_path_buf()]).filter(|p| p.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_and_links() {
        let stems = vec![(squash("Agent-Memory-Protocol"), "Agent-Memory-Protocol".to_string())];
        assert_eq!(close_match("agent memory protocol", &stems).as_deref(), Some("Agent-Memory-Protocol"));
        assert_eq!(close_match("Agent-Memry-Protocol", &stems).as_deref(), Some("Agent-Memory-Protocol"));
        assert_eq!(close_match("Unrelated", &stems), None);
        assert_eq!(replace_link("a [[x]] b [[x|y]] c [[x#h]] [[xx]]", "[[x]]", "[[Z]]"), "a [[Z]] b [[Z|y]] c [[Z#h]] [[xx]]");
    }

    #[test]
    fn diff() {
        let (r, a) = line_diff("a\nb\nc", "a\nc\nd");
        assert_eq!(r, vec!["b"]);
        assert_eq!(a, vec!["d"]);
    }
}
