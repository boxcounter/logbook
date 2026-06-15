use std::path::PathBuf;

/// Resolve root_path from --root-path flag or GUI's persisted root_path.txt.
///
/// Priority:
/// 1. `flag` (--root-path / -r)
/// 2. `root_path.txt` in macOS app local data dir
/// 3. None → caller prints error and exits
///
/// macOS app data dir: ~/Library/Application Support/com.logbook/
///
/// The bundle ID is determined by the Tauri config; typical default is
/// `com.tauri.dev` in dev. We check a few common names.
pub fn resolve_root_path(flag: Option<String>) -> Option<PathBuf> {
    if let Some(ref p) = flag {
        let path = PathBuf::from(p);
        if path.exists() && path.is_dir() {
            return Some(path);
        }
        eprintln!(
            "Warning: --root-path '{}' does not exist or is not a directory",
            p
        );
        return None;
    }

    // Try common macOS app data dirs for root_path.txt
    let home = std::env::var("HOME").ok()?;
    let candidates = [
        "Library/Application Support/com.logbook/root_path.txt",
        "Library/Application Support/com.tauri.dev/root_path.txt",
    ];

    for candidate in &candidates {
        let p = PathBuf::from(&home).join(candidate);
        if p.exists() {
            if let Ok(content) = std::fs::read_to_string(&p) {
                let trimmed = content.trim();
                let path = PathBuf::from(trimmed);
                if path.exists() && path.is_dir() {
                    return Some(path);
                }
            }
        }
    }

    None
}
