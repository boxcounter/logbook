use crate::models::{DayFile, Entry, MonthlyFile, Config};
use std::fs;
use std::path::{Path, PathBuf};

/// Day file path: {root}/{year}/{month:02}/{date}.md
/// date format: "2026-06-12". Date is canonical from filename, not stored in frontmatter.
pub fn day_path(root: &Path, date: &str) -> PathBuf {
    let parts: Vec<&str> = date.split('-').collect();
    let year = parts.get(0).unwrap_or(&"0000");
    let month = parts.get(1).unwrap_or(&"00");
    root.join(year).join(month).join(format!("{}.md", date))
}

/// Monthly file path: {root}/{year}/{month:02}/_monthly.md
pub fn monthly_path(root: &Path, year: i32, month: u32) -> PathBuf {
    root.join(year.to_string())
        .join(format!("{:02}", month))
        .join("_monthly.md")
}

/// Config path: {root}/config.yaml
pub fn config_path(root: &Path) -> PathBuf {
    root.join("config.yaml")
}

/// Read a day file. Returns empty DayFile if file doesn't exist.
pub fn read_day_file(root: &Path, date: &str) -> Result<DayFile, String> {
    let path = day_path(root, date);
    if !path.exists() {
        return Ok(DayFile { note: None, entries: vec![] });
    }
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
    parse_frontmatter::<DayFile>(&content)
        .map_err(|e| format!("Failed to parse {}: {}", path.display(), e))
}

/// Write a full day file (atomic: temp then rename).
pub fn write_day_file(root: &Path, date: &str, day_file: &DayFile) -> Result<(), String> {
    let path = day_path(root, date);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create directory: {}", e))?;
    }
    let yaml_body = yaml_serde::to_string(day_file)
        .map_err(|e| format!("Failed to serialize: {}", e))?;
    let content = format!("---\n{}---\n", yaml_body);
    let tmp_path = path.with_extension("tmp");
    fs::write(&tmp_path, &content)
        .map_err(|e| format!("Failed to write temp file: {}", e))?;
    fs::rename(&tmp_path, &path)
        .map_err(|e| format!("Failed to rename temp file: {}", e))?;
    Ok(())
}

/// Append an entry to a day file. Creates directories if needed.
pub fn append_to_day_file(root: &Path, date: &str, entry: &Entry) -> Result<Entry, String> {
    let mut day_file = read_day_file(root, date)?;
    let entry = entry.clone();
    day_file.entries.push(entry.clone());
    write_day_file(root, date, &day_file)?;
    Ok(entry)
}

/// Append entry from NewEntry (for integration tests and internal use).
pub fn append_new_entry(root: &Path, date: &str, new_entry: &crate::models::NewEntry) -> Result<Entry, String> {
    let duration = crate::commands::parse_duration(&new_entry.duration)?;
    let entry = Entry {
        id: uuid::Uuid::new_v4().to_string(),
        item: new_entry.item.clone(),
        duration,
        dimensions: new_entry.dimensions.clone(),
    };
    append_to_day_file(root, date, &entry)
}

/// Update an entry by ID. Applies only the fields present in `update`.
pub fn update_entry_in_file(root: &Path, date: &str, entry_id: &str, update: &crate::models::UpdateEntry) -> Result<DayFile, String> {
    let mut day_file = read_day_file(root, date)?;
    let pos = day_file.entries.iter().position(|e| e.id == entry_id)
        .ok_or_else(|| format!("Entry {} not found", entry_id))?;
    let entry = &mut day_file.entries[pos];
    if let Some(ref item) = update.item { entry.item = item.clone(); }
    if let Some(ref dur_str) = update.duration {
        entry.duration = crate::commands::parse_duration(dur_str)
            .map_err(|e| format!("Invalid duration: {}", e))?;
    }
    if let Some(ref dims) = update.dimensions { entry.dimensions = dims.clone(); }
    write_day_file(root, date, &day_file)?;
    Ok(day_file)
}

/// Delete an entry by ID from a day file.
pub fn delete_entry_from_file(root: &Path, date: &str, entry_id: &str) -> Result<DayFile, String> {
    let mut day_file = read_day_file(root, date)?;
    let pos = day_file.entries.iter().position(|e| e.id == entry_id)
        .ok_or_else(|| format!("Entry {} not found", entry_id))?;
    day_file.entries.remove(pos);
    write_day_file(root, date, &day_file)?;
    Ok(day_file)
}

/// Set the day note.
pub fn set_day_note_in_file(root: &Path, date: &str, note: &str) -> Result<DayFile, String> {
    let mut day_file = read_day_file(root, date)?;
    day_file.note = if note.is_empty() { None } else { Some(note.to_string()) };
    write_day_file(root, date, &day_file)?;
    Ok(day_file)
}

/// Read monthly file. Returns empty MonthlyFile if not found.
pub fn read_monthly_file(root: &Path, year: i32, month: u32) -> Result<MonthlyFile, String> {
    let path = monthly_path(root, year, month);
    if !path.exists() {
        return Ok(MonthlyFile { commitments: vec![] });
    }
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
    parse_frontmatter::<MonthlyFile>(&content)
        .map_err(|e| format!("Failed to parse {}: {}", path.display(), e))
}

/// Read config.yaml. Returns error if file missing.
pub fn read_config(root: &Path) -> Result<Config, String> {
    let path = config_path(root);
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
    yaml_serde::from_str::<Config>(&content)
        .map_err(|e| format!("Failed to parse {}: {}", path.display(), e))
}

/// Root path persistence (atomic write)
pub fn save_root_path(app_data_dir: &Path, root_path: &Path) -> Result<(), String> {
    let path = app_data_dir.join("root_path.txt");
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, root_path.to_string_lossy().as_ref())
        .map_err(|e| format!("Failed to save root path: {}", e))?;
    fs::rename(&tmp, &path)
        .map_err(|e| format!("Failed to rename root path file: {}", e))
}

/// Read persisted root path
pub fn read_root_path(app_data_dir: &Path) -> Option<PathBuf> {
    let path = app_data_dir.join("root_path.txt");
    if path.exists() {
        fs::read_to_string(&path)
            .ok()
            .map(|s| PathBuf::from(s.trim()))
    } else {
        None
    }
}

/// Parse YAML frontmatter: extract between --- delimiters.
/// Second `---` must appear at line start (or end of file) to avoid
/// matching horizontal rules in Markdown body.
fn parse_frontmatter<T: serde::de::DeserializeOwned>(content: &str) -> Result<T, String> {
    let content = content.trim();
    if !content.starts_with("---") {
        return Err("No frontmatter found".to_string());
    }
    let after_first = &content[3..];
    // Find `\n---` (second delimiter at line start) or end of string
    let end = after_first.find("\n---").unwrap_or_else(|| {
        if after_first.starts_with("---") { 0 } else { after_first.len() }
    });
    let yaml_str = if end == 0 { "" } else { &after_first[..end] }.trim();
    if yaml_str.is_empty() {
        return yaml_serde::from_str("{}").map_err(|e| format!("YAML parse error: {}", e));
    }
    yaml_serde::from_str(yaml_str).map_err(|e| format!("YAML parse error: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Entry;
    use std::collections::HashMap;

    #[test]
    fn test_parse_frontmatter_basic() {
        let input = "---\nentries: []\n---\n";
        let df: DayFile = parse_frontmatter(input).unwrap();
        assert!(df.entries.is_empty());
        assert!(df.note.is_none());
    }

    #[test]
    fn test_parse_frontmatter_with_note() {
        let input = "---\nnote: \"test note\"\nentries: []\n---\n";
        let df: DayFile = parse_frontmatter(input).unwrap();
        assert_eq!(df.note, Some("test note".to_string()));
    }

    #[test]
    fn test_day_path() {
        let root = Path::new("/data");
        let p = day_path(root, "2026-06-12");
        assert_eq!(p, PathBuf::from("/data/2026/06/2026-06-12.md"));
    }

    #[test]
    fn test_monthly_path() {
        let root = Path::new("/data");
        let p = monthly_path(root, 2026, 6);
        assert_eq!(p, PathBuf::from("/data/2026/06/_monthly.md"));
    }

    #[test]
    fn test_config_path() {
        let root = Path::new("/data");
        let p = config_path(root);
        assert_eq!(p, PathBuf::from("/data/config.yaml"));
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
            dimensions: HashMap::new(),
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

        let e1 = Entry { id: "id-a".into(), item: "A".into(), duration: 10, dimensions: HashMap::new() };
        let e2 = Entry { id: "id-b".into(), item: "B".into(), duration: 20, dimensions: HashMap::new() };
        append_to_day_file(&tmp, "2026-06-12", &e1).unwrap();
        append_to_day_file(&tmp, "2026-06-12", &e2).unwrap();

        let update = crate::models::UpdateEntry { item: Some("B-modified".into()), duration: None, dimensions: None };
        let df = update_entry_in_file(&tmp, "2026-06-12", "id-b", &update).unwrap();
        assert_eq!(df.entries[1].item, "B-modified");
        assert_eq!(df.entries[1].duration, 20); // unchanged

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_delete_entry() {
        let tmp = std::env::temp_dir().join("logbook_test_delete");
        let _ = fs::remove_dir_all(&tmp);

        let e1 = Entry { id: "id-a".into(), item: "A".into(), duration: 10, dimensions: HashMap::new() };
        let e2 = Entry { id: "id-b".into(), item: "B".into(), duration: 20, dimensions: HashMap::new() };
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
}
