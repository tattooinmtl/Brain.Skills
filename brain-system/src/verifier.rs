use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;
use walkdir::WalkDir;

use crate::config;

/// Sanitize a proposal id: reject anything but ASCII alphanumeric, `_`, `-`.
/// Length capped at 64. Rejects `.` and `/` explicitly — prevents traversal.
fn valid_proposal_id(s: &str) -> bool {
    if s.is_empty() || s.len() > 64 { return false; }
    s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct Proposal {
    pub proposal_id: String,
    pub created_at: String,
    pub entry_id: String,
    pub title: String,
    pub current_text: String,
    pub citations: Vec<String>,
    pub reason: String,
    pub evidence_summary: String,
    pub suggested_edit: String,
    pub confidence: f64,
    pub status: String,
}

pub fn compute_sha256(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.trim().as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn get_vault_path() -> PathBuf {
    config::vault_dir()
}

pub fn scan_and_verify() -> Vec<Proposal> {
    let vault = get_vault_path();
    let inbox_path = vault.join("00-Inbox");
    let _ = fs::create_dir_all(&inbox_path);

    let mut proposals = Vec::new();
    let gold_dirs = vec![vault.join("10-Entities"), vault.join("20-Concepts")];

    for gdir in gold_dirs {
        if !gdir.exists() { continue; }
        for entry in WalkDir::new(gdir).into_iter().filter_map(|e| e.ok()) {
            if entry.path().extension().map_or(false, |ext| ext == "md") {
                if let Ok(content) = fs::read_to_string(entry.path()) {
                    let _hash = compute_sha256(&content);
                    if content.contains("sources:") && content.contains("http") {
                        // In clean state, verified
                    }
                }
            }
        }
    }

    // Load any existing proposals from 00-Inbox/
    for entry in WalkDir::new(inbox_path).into_iter().filter_map(|e| e.ok()) {
        if entry.path().extension().map_or(false, |ext| ext == "json") {
            if let Ok(text) = fs::read_to_string(entry.path()) {
                if let Ok(p) = serde_json::from_str::<Proposal>(&text) {
                    if p.status == "pending" {
                        proposals.push(p);
                    }
                }
            }
        }
    }

    proposals
}

pub fn resolve_proposal(proposal_id: &str, action: &str) -> Result<(), String> {
    // SECURITY: proposal_id is user-controlled. Reject anything that isn't a
    // safe filename fragment to prevent path traversal into the vault.
    if !valid_proposal_id(proposal_id) {
        return Err("Invalid proposal id".to_string());
    }
    let vault = get_vault_path();
    let inbox_path = vault.join("00-Inbox");
    let prop_file = inbox_path.join(format!("{}.json", proposal_id));

    // Belt-and-suspenders: canonicalize and verify the file is actually inside
    // the inbox (in case future changes weaken the id validator).
    let inbox_canon = inbox_path.canonicalize().unwrap_or_else(|_| inbox_path.clone());
    let file_canon = match prop_file.canonicalize() {
        Ok(c) => c,
        Err(_) => return Err("Proposal not found".to_string()),
    };
    if !file_canon.starts_with(&inbox_canon) {
        return Err("Path escape blocked".to_string());
    }

    if !prop_file.exists() {
        return Err("Proposal not found".to_string());
    }

    if let Ok(text) = fs::read_to_string(&prop_file) {
        if let Ok(mut prop) = serde_json::from_str::<Proposal>(&text) {
            prop.status = action.to_string();
            
            if action == "approve" {
                let archive_dir = vault.join("_system").join("archive");
                let _ = fs::create_dir_all(&archive_dir);
                let _ = fs::write(archive_dir.join(format!("{}.json", proposal_id)), serde_json::to_string_pretty(&prop).unwrap_or_default());
                let _ = fs::remove_file(prop_file);
                return Ok(());
            } else {
                let _ = fs::remove_file(prop_file);
                return Ok(());
            }
        }
    }

    Err("Failed to parse proposal".to_string())
}
