//! Small shared helpers: hashing, URL decoding, path normalization, time.

use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    format!("{:x}", h.finalize())
}

pub fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

pub fn mtime_ms(p: &Path) -> i64 {
    std::fs::metadata(p)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Percent-decode operating on bytes, so multi-byte UTF-8 (accented note
/// names) survives. Invalid escapes are kept literally.
pub fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'%' if i + 2 < b.len() => {
                let hex = std::str::from_utf8(&b[i + 1..i + 3]).ok().and_then(|h| u8::from_str_radix(h, 16).ok());
                match hex {
                    Some(v) => { out.push(v); i += 3; }
                    None => { out.push(b'%'); i += 1; }
                }
            }
            b'+' => { out.push(b' '); i += 1; }
            c => { out.push(c); i += 1; }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Split `/path?a=1&b=2` into the path and decoded query pairs.
pub fn split_url(url: &str) -> (String, Vec<(String, String)>) {
    let (path, query) = match url.find('?') {
        Some(i) => (&url[..i], &url[i + 1..]),
        None => (url, ""),
    };
    let params = query
        .split('&')
        .filter(|kv| !kv.is_empty())
        .map(|kv| match kv.find('=') {
            Some(i) => (percent_decode(&kv[..i]), percent_decode(&kv[i + 1..])),
            None => (percent_decode(kv), String::new()),
        })
        .collect();
    (path.to_string(), params)
}

pub fn param<'a>(params: &'a [(String, String)], key: &str) -> Option<&'a str> {
    params.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
}

/// Lowercase, forward-slash form used to compare paths that came from
/// different tools (transcripts, walkdir, user input).
pub fn norm_path(p: &str) -> String {
    let mut s = p.replace('\\', "/").to_lowercase();
    if let Some(rest) = s.strip_prefix("//?/") { s = rest.to_string(); }
    // git-bash style /c/foo -> c:/foo
    let bytes = s.as_bytes();
    if bytes.len() >= 3 && bytes[0] == b'/' && bytes[1].is_ascii_alphabetic() && bytes[2] == b'/' {
        s = format!("{}:{}", bytes[1] as char, &s[2..]);
    }
    while s.ends_with('/') && s.len() > 3 { s.pop(); }
    s
}

pub fn display_path(p: &Path) -> String {
    let s = p.to_string_lossy().replace('\\', "/");
    s.strip_prefix("//?/").map(|x| x.to_string()).unwrap_or(s)
}

/// Canonicalize and confirm `candidate` lives under one of `roots`.
pub fn sandboxed(candidate: &str, roots: &[PathBuf]) -> Option<PathBuf> {
    if candidate.contains("..") { return None; }
    let canon = PathBuf::from(candidate).canonicalize().ok()?;
    for r in roots {
        if let Ok(rc) = r.canonicalize() {
            if canon.starts_with(&rc) { return Some(canon); }
        }
    }
    None
}

/// Truncate on a char boundary, appending an ellipsis when cut.
pub fn clip(s: &str, max: usize) -> String {
    let s = s.trim();
    if s.chars().count() <= max { return s.to_string(); }
    let mut out: String = s.chars().take(max).collect();
    out.push('…');
    out
}

/// Collapse whitespace runs so one-line summaries stay one line.
pub fn one_line(s: &str, max: usize) -> String {
    let joined = s.split_whitespace().collect::<Vec<_>>().join(" ");
    clip(&joined, max)
}

/// Parse an RFC3339 timestamp ("2026-09-05T18:50:11.123Z") to epoch ms.
pub fn parse_ts(s: &str) -> Option<i64> {
    let b = s.as_bytes();
    if b.len() < 19 { return None; }
    let num = |r: std::ops::Range<usize>| -> Option<i64> { s.get(r)?.parse().ok() };
    let (y, mo, d) = (num(0..4)?, num(5..7)?, num(8..10)?);
    let (h, mi, se) = (num(11..13)?, num(14..16)?, num(17..19)?);
    let mut ms = 0i64;
    if b.len() > 20 && b[19] == b'.' {
        let frac: String = s[20..].chars().take_while(|c| c.is_ascii_digit()).take(3).collect();
        if let Ok(v) = frac.parse::<i64>() { ms = v * 10i64.pow(3 - frac.len() as u32); }
    }
    let days = days_from_civil(y, mo as u32, d as u32);
    Some(((days * 86_400 + h * 3600 + mi * 60 + se) * 1000) + ms)
}

fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as i64;
    let mp = if m > 2 { m - 3 } else { m + 9 } as i64;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Epoch ms -> "2026-10-07T19:07:00Z".
pub fn iso(ms: i64) -> String {
    let secs = ms.div_euclid(1000);
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", y, m, d, rem / 3600, (rem / 60) % 60, rem % 60)
}

/// Write via temp file + rename so a crash never leaves a half-written note.
pub fn atomic_write(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() { std::fs::create_dir_all(parent)?; }
    let tmp = path.with_extension(format!(
        "{}.brain-tmp",
        path.extension().map(|e| e.to_string_lossy().to_string()).unwrap_or_default()
    ));
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_utf8_and_bad_escapes() {
        assert_eq!(percent_decode("caf%C3%A9"), "café");
        assert_eq!(percent_decode("100%"), "100%");
        assert_eq!(percent_decode("a%zzb"), "a%zzb");
        assert_eq!(percent_decode("a+b%2Fc"), "a b/c");
    }

    #[test]
    fn ts_roundtrip() {
        let ms = parse_ts("2026-09-05T18:50:11.120Z").unwrap();
        assert_eq!(iso(ms), "2026-09-05T18:50:11Z");
        assert_eq!(ms % 1000, 120);
    }

    #[test]
    fn norm_paths() {
        assert_eq!(norm_path(r"C:\.skills\Memory\"), "c:/.skills/memory");
        assert_eq!(norm_path("/c/.skills/memory"), "c:/.skills/memory");
        assert_eq!(norm_path(r"\\?\C:\x"), "c:/x");
    }
}
