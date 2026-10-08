//! Manual edits that don't belong in a source file: renamed labels, custom
//! colours, notes, and the append-only entry log on any node (e.g. a skill
//! revisited by a later session). Stored in `<vault>/_system/brain-graph/overlay.json`.

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::{config, util};

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct NodeEdit {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub label: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub color: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub note: String,
    #[serde(default)]
    pub updated: i64,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Entry {
    pub ts: i64,
    pub text: String,
    #[serde(default)]
    pub source: String,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Overlay {
    #[serde(default)]
    pub nodes: BTreeMap<String, NodeEdit>,
    #[serde(default)]
    pub entries: BTreeMap<String, Vec<Entry>>,
}

fn path() -> PathBuf {
    config::system_dir().join("brain-graph").join("overlay.json")
}

impl Overlay {
    pub fn load() -> Overlay {
        std::fs::read_to_string(path()).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
    }

    pub fn save(&self) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self).unwrap_or_default();
        util::atomic_write(&path(), json.as_bytes())
    }

    /// Empty strings clear a field. Returns false if the colour is malformed.
    pub fn set_edit(&mut self, id: &str, label: Option<&str>, color: Option<&str>, note: Option<&str>) -> bool {
        if let Some(c) = color {
            if !c.is_empty() && !valid_color(c) { return false; }
        }
        let e = self.nodes.entry(id.to_string()).or_default();
        if let Some(l) = label { e.label = util::clip(l, 120); }
        if let Some(c) = color { e.color = c.to_string(); }
        if let Some(n) = note { e.note = util::clip(n, 20_000); }
        e.updated = util::now_ms();
        if e.label.is_empty() && e.color.is_empty() && e.note.is_empty() { self.nodes.remove(id); }
        true
    }

    pub fn add_entry(&mut self, id: &str, text: &str, source: &str) {
        self.entries.entry(id.to_string()).or_default().push(Entry {
            ts: util::now_ms(),
            text: util::clip(text, 20_000),
            source: source.to_string(),
        });
    }
}

fn valid_color(c: &str) -> bool {
    let h = c.strip_prefix('#').unwrap_or("");
    (h.len() == 6 || h.len() == 3) && h.chars().all(|x| x.is_ascii_hexdigit())
}
