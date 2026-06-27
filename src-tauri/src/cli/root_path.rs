use std::path::PathBuf;

/// The macOS app data dir the GUI uses: `~/Library/Application Support/<bundle_id>`.
///
/// Bundle ID is injected at compile time via `LOGBOOK_CLI_BUNDLE_ID`, matching
/// the Tauri config selection (debug → com.boxcounter.logbook.dev,
/// release → com.boxcounter.logbook). This is the single source of truth for
/// both `root_path.txt` lookup and the shared `logbook.log` location.
pub fn app_data_dir() -> Option<PathBuf> {
    let bundle_id = env!("LOGBOOK_CLI_BUNDLE_ID");
    let home = std::env::var("HOME").ok()?;
    Some(
        PathBuf::from(&home)
            .join("Library/Application Support")
            .join(bundle_id),
    )
}

/// Resolve root_path from --root-path flag or GUI's persisted root_path.txt.
///
/// Priority:
/// 1. `flag` (--root-path / -r)
/// 2. `root_path.txt` in the bundle-specific macOS app data dir
/// 3. None → caller prints error and exits
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

    let root_path_txt = app_data_dir()?.join("root_path.txt");

    if root_path_txt.exists() {
        if let Ok(content) = std::fs::read_to_string(&root_path_txt) {
            let trimmed = content.trim();
            let path = PathBuf::from(trimmed);
            if path.exists() && path.is_dir() {
                return Some(path);
            }
            eprintln!(
                "Warning: root_path.txt points to '{}', which does not exist or is not a directory",
                trimmed
            );
            return None;
        }
    }

    let bundle_id = env!("LOGBOOK_CLI_BUNDLE_ID");
    eprintln!(
        "Info: No root_path.txt found for {bundle_id}.\n\
         Run the {app} GUI once to initialize, or use --root-path to specify manually.",
        bundle_id = bundle_id,
        app = if bundle_id.ends_with(".dev") { "Logbook (dev)" } else { "Logbook" }
    );
    None
}
