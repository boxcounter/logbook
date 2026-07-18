//! Path helpers, atomic I/O (`atomic_write`, PID-suffixed tmp names), root_path
//! persistence, pure-YAML day file read/write, template/month dimensions.
//!
//! Global state and root-switch lifecycle (AGENTS.md「数据安全与可靠性」):
//! - `FILE_LOCKS` / `RECENTLY_APP_WRITTEN` — keyed by absolute file path (the
//!   path embeds the data root), hence root-agnostic: entries for a previous
//!   root simply go stale. No reset is needed or triggered on root switch.

use crate::models::{Commitment, DayFile, Dimension, Entry, Template};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{Duration, Instant};

/// Per-file mutexes for in-process mutual exclusion. Keys accumulate over the
/// lifetime of the process (one entry per distinct day file accessed). Growth
/// is bounded by the total number of distinct dates ever opened in a session
/// (<1 MiB/year for a heavy user); a periodic sweep is not needed.
static FILE_LOCKS: LazyLock<Mutex<HashMap<PathBuf, Arc<Mutex<()>>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Day file paths recently written by the app itself. Used by the file watcher
/// to suppress reload events for its own writes (avoid redundant frontend
/// reloads when the app just updated the data).
static RECENTLY_APP_WRITTEN: LazyLock<Mutex<HashMap<PathBuf, Instant>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Mark a day file path as recently written by the app. The watcher will
/// skip events for this path for a short cooldown window.
pub fn mark_file_written_by_app(path: &Path) {
    if let Ok(mut map) = RECENTLY_APP_WRITTEN.lock() {
        map.insert(path.to_path_buf(), Instant::now());
        map.retain(|_, t| t.elapsed() < Duration::from_secs(5));
    }
}

/// Returns true if the file was written by the app within the cooldown window.
pub fn was_recently_written_by_app(path: &Path) -> bool {
    RECENTLY_APP_WRITTEN
        .lock()
        .map(|mut map| {
            map.retain(|_, t| t.elapsed() < Duration::from_secs(5));
            map.contains_key(path)
        })
        .unwrap_or(false)
}

fn with_file_lock<T, F: FnOnce() -> Result<T, String>>(path: &Path, f: F) -> Result<T, String> {
    let lock = {
        let mut map = FILE_LOCKS
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        map.entry(path.to_path_buf())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    };
    let _guard = lock.lock().unwrap_or_else(|e| e.into_inner());
    f()
}

/// Day file path: {root}/{year}/{month:02}/{date}.yaml
/// Validates date format before constructing path.
/// date format: "2026-06-12". Date is canonical from filename, not stored in frontmatter.
pub fn day_path(root: &Path, date: &str) -> Result<PathBuf, String> {
    use chrono::Datelike;
    let d = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|e| format!("Invalid date '{}': {}", date, e))?;
    // Derive the path from the parsed date so a lenient input like "2026-6-5"
    // still lands in the canonical zero-padded /YYYY/MM/YYYY-MM-DD.yaml that
    // monthly_path and every month scan use. Otherwise the file would be
    // written to /2026/6/ and silently missed by aggregation.
    Ok(root
        .join(format!("{:04}", d.year()))
        .join(format!("{:02}", d.month()))
        .join(format!("{:04}-{:02}-{:02}.yaml", d.year(), d.month(), d.day())))
}

/// Template path: {root}/template.yaml
pub fn template_path(root: &Path) -> PathBuf {
    root.join("template.yaml")
}

/// Dimensions template: {root}/dimensions.template.yaml
pub fn dimensions_template_path(root: &Path) -> PathBuf {
    root.join("dimensions.template.yaml")
}

/// Data version file: {root}/version.txt
pub fn version_path(root: &Path) -> PathBuf {
    root.join("version.txt")
}

/// Process-unique temp path for atomic writes: "<target>.tmp.<pid>" next to
/// the target file. Two processes writing the same target no longer share (and
/// truncate) a single deterministic .tmp file; the final rename stays atomic.
pub fn tmp_path_for(path: &Path) -> PathBuf {
    match path.file_name() {
        Some(name) => {
            let mut tmp = name.to_os_string();
            tmp.push(format!(".tmp.{}", std::process::id()));
            path.with_file_name(tmp)
        }
        None => path.with_extension(format!("tmp.{}", std::process::id())),
    }
}

/// Returns true if `name` looks like an atomic-write temp file: the legacy
/// "<target>.tmp" form or the current "<target>.tmp.<pid>" form.
pub fn is_tmp_file_name(name: &str) -> bool {
    if name.ends_with(".tmp") {
        return true;
    }
    if let Some(pos) = name.rfind(".tmp.") {
        let suffix = &name[pos + 5..];
        return !suffix.is_empty() && suffix.chars().all(|c| c.is_ascii_digit());
    }
    false
}

/// Atomic write: write to a process-unique tmp file in the same directory,
/// then rename over the target. Same-directory rename is atomic on macOS/Linux.
pub fn atomic_write(path: &Path, content: &str) -> Result<(), String> {
    let tmp_path = tmp_path_for(path);
    fs::write(&tmp_path, content)
        .map_err(|e| format!("Failed to write temp file {}: {}", tmp_path.display(), e))?;
    fs::rename(&tmp_path, path)
        .map_err(|e| format!("Failed to rename temp file {}: {}", path.display(), e))
}

/// Write version.txt (atomic: tmp then rename).
pub fn write_version_file(root: &Path, version: u32) -> Result<(), String> {
    let path = version_path(root);
    let content = version.to_string();
    atomic_write(&path, &content)
}

/// Read version.txt. Returns Ok(None) if file doesn't exist.
/// Returns Err if file exists but content is not a valid unsigned integer.
pub fn read_version_file(root: &Path) -> Result<Option<u32>, String> {
    let path = version_path(root);
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Err(format!(
            "version.txt is empty in {}",
            path.display()
        ));
    }
    trimmed
        .parse::<u32>()
        .map(Some)
        .map_err(|_| {
            format!(
                "version.txt contains invalid version '{}' in {}",
                trimmed,
                path.display()
            )
        })
}

/// Monthly dimensions: {root}/{year}/{month:02}/dimensions.yaml
pub fn dimensions_path(root: &Path, year: i32, month: u32) -> PathBuf {
    root.join(year.to_string())
        .join(format!("{:02}", month))
        .join("dimensions.yaml")
}

/// Monthly commitments: {root}/{year}/{month:02}/commitments.yaml
pub fn commitments_path(root: &Path, year: i32, month: u32) -> PathBuf {
    root.join(year.to_string())
        .join(format!("{:02}", month))
        .join("commitments.yaml")
}

/// Read a day file. Returns empty DayFile if file doesn't exist.
pub fn read_day_file(root: &Path, date: &str) -> Result<DayFile, String> {
    let path = day_path(root, date)?;
    if !path.exists() {
        return Ok(DayFile {
            note: None,
            entries: vec![],
        });
    }
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
    let content = content.trim_start_matches('\u{feff}');
    yaml_serde::from_str::<DayFile>(content)
        .map_err(|e| format!("Failed to parse {}: {}", path.display(), e))
}

/// Write a full day file (atomic: tmp then rename).
pub fn write_day_file(root: &Path, date: &str, day_file: &DayFile) -> Result<(), String> {
    let path = day_path(root, date)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create directory: {}", e))?;
    }
    let yaml_body =
        yaml_serde::to_string(day_file).map_err(|e| format!("Failed to serialize: {}", e))?;
    let tmp_path = tmp_path_for(&path);
    fs::write(&tmp_path, &yaml_body).map_err(|e| format!("Failed to write temp file: {}", e))?;
    fs::rename(&tmp_path, &path).map_err(|e| format!("Failed to rename temp file: {}", e))?;
    mark_file_written_by_app(&path);
    Ok(())
}

/// Append an entry to a day file. Creates directories if needed.
pub fn append_to_day_file(root: &Path, date: &str, entry: &Entry) -> Result<Entry, String> {
    let path = day_path(root, date)?;
    with_file_lock(&path, || {
        let mut day_file = read_day_file(root, date)?;
        let entry = entry.clone();
        day_file.entries.push(entry);
        write_day_file(root, date, &day_file)?;
        Ok(day_file.entries.last().unwrap().clone())
    })
}

/// Append entry from CreateEntryInput (for integration tests and internal use).
pub fn append_new_entry(
    root: &Path,
    date: &str,
    new_entry: &crate::models::CreateEntryInput,
) -> Result<Entry, String> {
    let duration = crate::commands::parse_duration(&new_entry.duration)?;
    let (year, month) = year_month_from_date(date)?;
    create_dimensions_if_missing(root, year, month)?;
    let dims = resolve_month_dimensions(root, year, month)?;
    crate::commands::validate_required_dimensions(&dims, &new_entry.dimensions)?;
    let entry = Entry {
        id: uuid::Uuid::new_v4().to_string(),
        item: new_entry.item.clone(),
        duration,
        dimensions: new_entry.dimensions.clone(),
    };
    append_to_day_file(root, date, &entry)
}

/// Update an entry by ID. Applies only the fields present in `update`.
pub fn update_entry_in_file(
    root: &Path,
    date: &str,
    entry_id: &str,
    update: &crate::models::UpdateEntryInput,
) -> Result<DayFile, String> {
    let path = day_path(root, date)?;
    with_file_lock(&path, || {
        let mut day_file = read_day_file(root, date)?;
        let pos = day_file
            .entries
            .iter()
            .position(|e| e.id == entry_id)
            .ok_or_else(|| format!("Entry {} not found", entry_id))?;
        let entry = &mut day_file.entries[pos];
        if let Some(ref item) = update.item {
            entry.item = item.clone();
        }
        if let Some(ref dur_str) = update.duration {
            entry.duration = crate::commands::parse_duration(dur_str)
                .map_err(|e| format!("Invalid duration: {}", e))?;
        }
        if let Some(ref dims) = update.dimensions {
            let (year, month) = year_month_from_date(date)?;
            let effective = resolve_month_dimensions(root, year, month)?;
            crate::commands::validate_required_dimensions(&effective, dims)?;
            entry.dimensions = dims.clone();
        }
        write_day_file(root, date, &day_file)?;
        Ok(day_file)
    })
}

/// Delete an entry by ID from a day file.
pub fn delete_entry_from_file(root: &Path, date: &str, entry_id: &str) -> Result<DayFile, String> {
    let path = day_path(root, date)?;
    with_file_lock(&path, || {
        let mut day_file = read_day_file(root, date)?;
        let pos = day_file
            .entries
            .iter()
            .position(|e| e.id == entry_id)
            .ok_or_else(|| format!("Entry {} not found", entry_id))?;
        day_file.entries.remove(pos);
        write_day_file(root, date, &day_file)?;
        Ok(day_file)
    })
}

/// Set the day note.
pub fn set_day_note_in_file(root: &Path, date: &str, note: &str) -> Result<DayFile, String> {
    let path = day_path(root, date)?;
    with_file_lock(&path, || {
        let mut day_file = read_day_file(root, date)?;
        day_file.note = if note.is_empty() {
            None
        } else {
            Some(note.to_string())
        };
        write_day_file(root, date, &day_file)?;
        Ok(day_file)
    })
}

/// Read a month's dimensions.yaml. Returns empty Vec if file doesn't exist.
pub fn read_dimensions_file(root: &Path, year: i32, month: u32) -> Result<Vec<Dimension>, String> {
    let path = dimensions_path(root, year, month);
    if !path.exists() {
        return Ok(vec![]);
    }
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
    let content = content.trim_start_matches('\u{feff}');
    // dimensions.yaml is a flat YAML array of Dimension objects (pure YAML, no frontmatter).
    yaml_serde::from_str::<Vec<Dimension>>(content)
        .map_err(|e| format!("Failed to parse {}: {}", path.display(), e))
}

/// Write dimensions to a month's dimensions.yaml (atomic: tmp then rename).
/// Writes pure YAML — no frontmatter `---` delimiters.
pub fn write_dimensions_file(
    root: &Path,
    year: i32,
    month: u32,
    dimensions: &[Dimension],
) -> Result<(), String> {
    let path = dimensions_path(root, year, month);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create directory: {}", e))?;
    }
    let yaml_body = yaml_serde::to_string(dimensions)
        .map_err(|e| format!("Failed to serialize dimensions: {}", e))?;
    let tmp_path = tmp_path_for(&path);
    fs::write(&tmp_path, yaml_body)
        .map_err(|e| format!("Failed to write temp file: {}", e))?;
    fs::rename(&tmp_path, &path)
        .map_err(|e| format!("Failed to rename temp file: {}", e))?;
    Ok(())
}

/// Read a month's commitments.yaml. Returns empty Vec if file doesn't exist.
pub fn read_commitments_file(
    root: &Path,
    year: i32,
    month: u32,
) -> Result<Vec<Commitment>, String> {
    let path = commitments_path(root, year, month);
    if !path.exists() {
        return Ok(vec![]);
    }
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
    let content = content.trim_start_matches('\u{feff}');
    // commitments.yaml is a flat YAML array of Commitment objects (pure YAML, no frontmatter).
    yaml_serde::from_str::<Vec<Commitment>>(content)
        .map_err(|e| format!("Failed to parse {}: {}", path.display(), e))
}

/// Write commitments to a month's commitments.yaml (atomic: tmp then rename).
/// Writes pure YAML — no frontmatter `---` delimiters.
pub fn write_commitments_file(
    root: &Path,
    year: i32,
    month: u32,
    commitments: &[Commitment],
) -> Result<(), String> {
    let path = commitments_path(root, year, month);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create directory: {}", e))?;
    }
    let yaml_body = yaml_serde::to_string(commitments)
        .map_err(|e| format!("Failed to serialize commitments: {}", e))?;
    let tmp_path = tmp_path_for(&path);
    fs::write(&tmp_path, yaml_body)
        .map_err(|e| format!("Failed to write temp file: {}", e))?;
    fs::rename(&tmp_path, &path)
        .map_err(|e| format!("Failed to rename temp file: {}", e))?;
    Ok(())
}

/// Read template.yaml. Returns error if file missing.
pub fn read_template(root: &Path) -> Result<Template, String> {
    let path = template_path(root);
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
    // Strip a leading UTF-8 BOM so a template.yaml saved by an external editor
    // that prepends one still parses (yaml_serde treats the BOM as content).
    let content = content.trim_start_matches('\u{feff}');
    yaml_serde::from_str::<Template>(content)
        .map_err(|e| format!("Failed to parse {}: {}", path.display(), e))
}

/// Read dimensions.template.yaml.
pub fn read_dimensions_template(root: &Path) -> Result<Template, String> {
    let path = dimensions_template_path(root);
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
    let content = content.trim_start_matches('\u{feff}');
    yaml_serde::from_str::<Template>(content)
        .map_err(|e| format!("Failed to parse {}: {}", path.display(), e))
}

/// Parse (year, month) from an ISO date string "YYYY-MM-DD".
pub fn year_month_from_date(date: &str) -> Result<(i32, u32), String> {
    use chrono::Datelike;
    let d = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|e| format!("Invalid date '{}': {}", date, e))?;
    Ok((d.year(), d.month()))
}

/// Effective dimensions for a month: the month's dimensions.yaml if it exists,
/// otherwise the dimensions.template.yaml. Tolerates missing files (returns
/// empty vec).
pub fn resolve_month_dimensions(
    root: &Path,
    year: i32,
    month: u32,
) -> Result<Vec<Dimension>, String> {
    let dims = read_dimensions_file(root, year, month)?;
    if !dims.is_empty() {
        return Ok(dims);
    }
    if !dimensions_template_path(root).exists() {
        return Ok(vec![]);
    }
    Ok(read_dimensions_template(root)?.dimensions)
}

/// Create dimensions.yaml from template if the month has no dimensions yet.
/// No-op if dimensions.yaml already exists or the template has no dimensions.
pub fn create_dimensions_if_missing(
    root: &Path,
    year: i32,
    month: u32,
) -> Result<(), String> {
    if dimensions_path(root, year, month).exists() {
        return Ok(());
    }
    if !dimensions_template_path(root).exists() {
        return Ok(());
    }
    let template_dims = read_dimensions_template(root)?.dimensions;
    if template_dims.is_empty() {
        return Ok(());
    }
    write_dimensions_file(root, year, month, &template_dims)
}

/// Remove orphaned .tmp files from the data tree (crashed mid-write).
pub fn cleanup_tmp_files(root: &Path) {
    const MAX_DEPTH: u32 = 5;
    fn recurse(dir: &Path, depth: u32) {
        if depth > MAX_DEPTH {
            crate::error_log::log_error(
                "cleanup_tmp_files",
                &format!("Max depth {} exceeded at {}", MAX_DEPTH, dir.display()),
            );
            return;
        }
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(e) => {
                crate::error_log::log_error(
                    "cleanup_tmp_files",
                    &format!("Failed to read directory {}: {}", dir.display(), e),
                );
                return;
            }
        };
        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    crate::error_log::log_error(
                        "cleanup_tmp_files",
                        &format!("Failed to read dir entry in {}: {:?}", dir.display(), e),
                    );
                    continue;
                }
            };
            let path = entry.path();
            if path.is_dir() {
                recurse(&path, depth + 1);
            } else if path
                .file_name()
                .and_then(|n| n.to_str())
                .map_or(false, is_tmp_file_name)
            {
                if let Err(e) = std::fs::remove_file(&path) {
                    crate::error_log::log_error(
                        "cleanup_tmp_files",
                        &format!("Failed to remove orphaned tmp file {}: {}", path.display(), e),
                    );
                }
            }
        }
    }
    recurse(root, 0);
}

/// Root path persistence (atomic write)
pub fn save_root_path(app_data_dir: &Path, root_path: &Path) -> Result<(), String> {
    let path = app_data_dir.join("root_path.txt");
    let tmp = tmp_path_for(&path);
    fs::write(&tmp, root_path.to_string_lossy().as_ref())
        .map_err(|e| format!("Failed to save root path: {}", e))?;
    fs::rename(&tmp, &path).map_err(|e| format!("Failed to rename root path file: {}", e))
}

/// Read persisted root path
pub fn read_root_path(app_data_dir: &Path) -> Option<PathBuf> {
    let path = app_data_dir.join("root_path.txt");
    if path.exists() {
        match fs::read_to_string(&path) {
            Ok(s) => Some(PathBuf::from(s.trim())),
            Err(e) => {
                crate::error_log::log_error(
                    "read_root_path",
                    &format!("Failed to read root_path.txt: {}", e),
                );
                None
            }
        }
    } else {
        None
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Entry;
    use std::collections::BTreeMap;

    #[test]
    fn test_entry_dimensions_serialize_deterministically() {
        // Entry.dimensions is a BTreeMap, so serialization is key-sorted and
        // independent of insertion order. This prevents spurious file diffs and
        // false verify_op_log content-mismatches on multi-dimension entries.
        let keys = ["goal", "business-line", "category", "importance-urgency"];
        let mk = |order: &[usize]| {
            let mut d = BTreeMap::new();
            for &i in order {
                d.insert(keys[i].to_string(), "v".to_string());
            }
            yaml_serde::to_string(&Entry {
                id: "x".into(),
                item: "i".into(),
                duration: 30,
                dimensions: d,
                    })
            .unwrap()
        };
        // Same keys inserted in different orders must serialize identically.
        let a = mk(&[0, 1, 2, 3]);
        let b = mk(&[3, 2, 1, 0]);
        let c = mk(&[2, 0, 3, 1]);
        assert_eq!(a, b);
        assert_eq!(a, c);
        // And the order is the sorted one.
        assert!(a.find("business-line").unwrap() < a.find("goal").unwrap());
    }

    #[test]
    fn test_day_path() {
        let root = Path::new("/data");
        let p = day_path(root, "2026-06-12").unwrap();
        assert_eq!(p, PathBuf::from("/data/2026/06/2026-06-12.yaml"));
    }

    #[test]
    fn test_day_path_normalizes_unpadded_date() {
        // chrono accepts "2026-6-5"; the path must still be the canonical padded
        // form so it lands in the same /2026/06/ dir that month scans look in.
        let root = Path::new("/data");
        let p = day_path(root, "2026-6-5").unwrap();
        assert_eq!(p, PathBuf::from("/data/2026/06/2026-06-05.yaml"));
    }

    #[test]
    fn test_day_path_invalid() {
        assert!(day_path(Path::new("/data"), "bad-date").is_err());
    }

    #[test]
    fn test_read_day_file_with_utf8_bom() {
        let tmp = std::env::temp_dir().join("logbook_test_bom");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let day_file = tmp.join("2026/07/2026-07-05.yaml");
        fs::create_dir_all(day_file.parent().unwrap()).unwrap();
        fs::write(&day_file, "\u{feff}note: bom note\nentries: []\n").unwrap();

        let df = read_day_file(&tmp, "2026-07-05").unwrap();
        assert_eq!(df.note, Some("bom note".to_string()));
        assert!(df.entries.is_empty());

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_template_path() {
        let root = Path::new("/data");
        let p = template_path(root);
        assert_eq!(p, PathBuf::from("/data/template.yaml"));
    }

    #[test]
    fn test_new_file_paths() {
        let root = Path::new("/data");
        assert_eq!(
            dimensions_template_path(root),
            PathBuf::from("/data/dimensions.template.yaml")
        );
        assert_eq!(
            dimensions_path(root, 2026, 6),
            PathBuf::from("/data/2026/06/dimensions.yaml")
        );
        assert_eq!(
            commitments_path(root, 2026, 6),
            PathBuf::from("/data/2026/06/commitments.yaml")
        );
    }

    #[test]
    fn test_version_path() {
        let root = Path::new("/data");
        let p = version_path(root);
        assert_eq!(p, PathBuf::from("/data/version.txt"));
    }

    #[test]
    fn test_write_and_read_version_roundtrip() {
        let tmp = std::env::temp_dir().join("logbook_test_version_rt");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        write_version_file(&tmp, 1).unwrap();
        let v = read_version_file(&tmp);
        assert_eq!(v, Ok(Some(1)));

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_read_version_file_not_found() {
        let tmp = std::env::temp_dir().join("logbook_test_version_nf");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let v = read_version_file(&tmp);
        assert_eq!(v, Ok(None));

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_read_version_file_invalid_content() {
        let tmp = std::env::temp_dir().join("logbook_test_version_invalid");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        // Empty file
        fs::write(tmp.join("version.txt"), "").unwrap();
        assert!(read_version_file(&tmp).is_err());

        // Non-integer content
        fs::write(tmp.join("version.txt"), "abc").unwrap();
        assert!(read_version_file(&tmp).is_err());

        // Whitespace-only
        fs::write(tmp.join("version.txt"), "  \n  ").unwrap();
        assert!(read_version_file(&tmp).is_err());

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_year_month_from_date() {
        assert_eq!(year_month_from_date("2026-07-15").unwrap(), (2026, 7));
    }

    #[test]
    fn test_year_month_from_date_invalid() {
        assert!(year_month_from_date("nope").is_err());
    }

    #[test]
    fn test_read_empty_day_file() {
        let tmp = std::env::temp_dir().join("logbook_test_empty2");
        let _ = fs::remove_dir_all(&tmp);
        let df = read_day_file(&tmp, "2026-06-12").unwrap();
        assert!(df.entries.is_empty());
        assert!(df.note.is_none());
    }

    #[test]
    fn test_append_and_read_roundtrip() {
        let tmp = std::env::temp_dir().join("logbook_test_rt2");
        let _ = fs::remove_dir_all(&tmp);

        let entry = Entry {
            id: uuid::Uuid::new_v4().to_string(),
            item: "Test".to_string(),
            duration: 30,
            dimensions: BTreeMap::new(),
            };
        append_to_day_file(&tmp, "2026-06-12", &entry).unwrap();

        let df = read_day_file(&tmp, "2026-06-12").unwrap();
        assert_eq!(df.entries.len(), 1);
        assert_eq!(df.entries[0].item, "Test");

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_update_entry() {
        let tmp = std::env::temp_dir().join("logbook_test_update");
        let _ = fs::remove_dir_all(&tmp);

        let e1 = Entry {
            id: "id-a".into(),
            item: "A".into(),
            duration: 10,
            dimensions: BTreeMap::new(),
            };
        let e2 = Entry {
            id: "id-b".into(),
            item: "B".into(),
            duration: 20,
            dimensions: BTreeMap::new(),
            };
        append_to_day_file(&tmp, "2026-06-12", &e1).unwrap();
        append_to_day_file(&tmp, "2026-06-12", &e2).unwrap();

        let update = crate::models::UpdateEntryInput {
            item: Some("B-modified".into()),
            duration: None,
            dimensions: None,
        };
        let df = update_entry_in_file(&tmp, "2026-06-12", "id-b", &update).unwrap();
        assert_eq!(df.entries[1].item, "B-modified");
        assert_eq!(df.entries[1].duration, 20); // unchanged

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_delete_entry() {
        let tmp = std::env::temp_dir().join("logbook_test_delete");
        let _ = fs::remove_dir_all(&tmp);

        let e1 = Entry {
            id: "id-a".into(),
            item: "A".into(),
            duration: 10,
            dimensions: BTreeMap::new(),
            };
        let e2 = Entry {
            id: "id-b".into(),
            item: "B".into(),
            duration: 20,
            dimensions: BTreeMap::new(),
            };
        append_to_day_file(&tmp, "2026-06-12", &e1).unwrap();
        append_to_day_file(&tmp, "2026-06-12", &e2).unwrap();

        let df = delete_entry_from_file(&tmp, "2026-06-12", "id-a").unwrap();
        assert_eq!(df.entries.len(), 1);
        assert_eq!(df.entries[0].item, "B");

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_set_day_note() {
        let tmp = std::env::temp_dir().join("logbook_test_note");
        let _ = fs::remove_dir_all(&tmp);

        let df = set_day_note_in_file(&tmp, "2026-06-12", "春节补班").unwrap();
        assert_eq!(df.note, Some("春节补班".to_string()));

        let df2 = read_day_file(&tmp, "2026-06-12").unwrap();
        assert_eq!(df2.note, Some("春节补班".to_string()));

        // Clear note
        let df3 = set_day_note_in_file(&tmp, "2026-06-12", "").unwrap();
        assert_eq!(df3.note, None);

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_cleanup_tmp_files_removes_orphaned_tmp() {
        let tmp = std::env::temp_dir().join("logbook_test_cleanup_tmp");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        // Create an orphaned .tmp file (simulating crash during atomic write)
        let day_dir = tmp.join("2026/07");
        fs::create_dir_all(&day_dir).unwrap();
        fs::write(day_dir.join("2026-07-04.yaml.tmp"), "data").unwrap();
        // Also create a valid .yaml file that should survive
        fs::write(day_dir.join("2026-07-04.yaml"), "note:\nentries: []\n").unwrap();

        cleanup_tmp_files(&tmp);

        assert!(!day_dir.join("2026-07-04.yaml.tmp").exists(), "orphaned .tmp should be removed");
        assert!(day_dir.join("2026-07-04.yaml").exists(), "valid .yaml should survive");

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_tmp_path_for_appends_pid_suffix() {
        let p = PathBuf::from("/data/2026/07/2026-07-04.yaml");
        let tmp = tmp_path_for(&p);
        let name = tmp.file_name().unwrap().to_string_lossy().to_string();
        assert!(
            name.starts_with("2026-07-04.yaml.tmp."),
            "tmp path must keep the target name and append .tmp.<pid>, got {}",
            name
        );
        let suffix = name.trim_start_matches("2026-07-04.yaml.tmp.");
        assert!(
            !suffix.is_empty() && suffix.chars().all(|c| c.is_ascii_digit()),
            "suffix must be a numeric pid, got {}",
            suffix
        );
        assert_eq!(tmp.parent(), p.parent());
    }

    #[test]
    fn test_is_tmp_file_name_recognizes_legacy_and_suffixed() {
        assert!(is_tmp_file_name("2026-07-04.yaml.tmp"));
        assert!(is_tmp_file_name("2026-07-04.yaml.tmp.12345"));
        assert!(is_tmp_file_name("version.txt.tmp.7"));
        assert!(!is_tmp_file_name("2026-07-04.yaml"));
        assert!(!is_tmp_file_name("2026-07-04.yaml.tmpx"));
        assert!(!is_tmp_file_name("2026-07-04.yaml.tmp."));
        assert!(!is_tmp_file_name("2026-07-04.yaml.tmp.abc"));
    }

    #[test]
    fn test_cleanup_tmp_files_removes_suffixed_tmp() {
        let tmp = std::env::temp_dir().join("logbook_test_cleanup_suffixed_tmp");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        // Orphaned tmp file with the process-unique suffix form "<name>.tmp.<pid>"
        let day_dir = tmp.join("2026/07");
        fs::create_dir_all(&day_dir).unwrap();
        fs::write(day_dir.join("2026-07-04.yaml.tmp.12345"), "data").unwrap();
        fs::write(day_dir.join("2026-07-04.yaml.tmp"), "legacy").unwrap();
        fs::write(day_dir.join("2026-07-04.yaml"), "note:\nentries: []\n").unwrap();

        cleanup_tmp_files(&tmp);

        assert!(
            !day_dir.join("2026-07-04.yaml.tmp.12345").exists(),
            "suffixed orphaned tmp should be removed"
        );
        assert!(
            !day_dir.join("2026-07-04.yaml.tmp").exists(),
            "legacy orphaned tmp should be removed"
        );
        assert!(day_dir.join("2026-07-04.yaml").exists(), "valid .yaml should survive");

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_atomic_writes_leave_no_tmp_residue() {
        let tmp = std::env::temp_dir().join("logbook_test_atomic_no_residue");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        write_version_file(&tmp, 2).unwrap();
        write_day_file(
            &tmp,
            "2026-07-04",
            &DayFile {
                note: None,
                entries: vec![],
            },
        )
        .unwrap();
        write_dimensions_file(&tmp, 2026, 7, &[]).unwrap();
        write_commitments_file(&tmp, 2026, 7, &[]).unwrap();

        // Walk the tree: no tmp file (legacy or suffixed) may remain.
        fn assert_no_tmp(dir: &Path) {
            for entry in fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    assert_no_tmp(&path);
                } else {
                    let name = path.file_name().unwrap().to_string_lossy().to_string();
                    assert!(
                        !is_tmp_file_name(&name),
                        "tmp residue left behind: {}",
                        path.display()
                    );
                }
            }
        }
        assert_no_tmp(&tmp);

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_save_root_path_atomic_and_read_roundtrip() {
        let tmp = std::env::temp_dir().join("logbook_test_root_path");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let data_root = tmp.join("my-data");
        save_root_path(&tmp, &data_root).unwrap();
        let read = read_root_path(&tmp).unwrap();
        assert_eq!(read, data_root);

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_read_root_path_nonexistent_returns_none() {
        let tmp = std::env::temp_dir().join("logbook_test_root_path_nonexist");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        assert_eq!(read_root_path(&tmp), None);

        let _ = fs::remove_dir_all(&tmp);
    }
}
