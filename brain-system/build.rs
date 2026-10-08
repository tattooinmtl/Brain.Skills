// Stamps the build date into the binary so /about and /api/version never
// show a stale hardcoded release date.
use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (y, m, d) = civil_from_days((secs / 86_400) as i64);
    const MONTHS: [&str; 12] = [
        "January", "February", "March", "April", "May", "June", "July",
        "August", "September", "October", "November", "December",
    ];
    println!(
        "cargo:rustc-env=BRAIN_BUILD_DATE={} {}, {}",
        MONTHS[(m - 1) as usize], d, y
    );
    println!("cargo:rustc-env=BRAIN_BUILD_YEAR={}", y);
    println!("cargo:rerun-if-changed=src");
}

// Howard Hinnant's days-from-epoch -> (year, month, day).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}
