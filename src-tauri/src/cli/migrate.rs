use std::fs;
use std::path::Path;

pub fn run(root: &Path) -> Result<(), String> {
    eprintln!("Scanning for .md day files in {}...", root.display());

    let mut converted = 0u32;
    let mut skipped = 0u32;
    let mut errors: Vec<String> = Vec::new();

    // Walk {root}/{YYYY}/{MM}/*.md
    match fs::read_dir(root) {
        Ok(year_entries) => {
            for year_entry in year_entries {
                let year_entry = match year_entry {
                    Ok(e) => e,
                    Err(e) => {
                        errors.push(format!("Failed to read dir entry: {}", e));
                        continue;
                    }
                };
                let year_path = year_entry.path();
                if !year_path.is_dir() {
                    continue;
                }
                let year_name = year_entry.file_name().to_string_lossy().into_owned();
                if year_name.parse::<i32>().is_err() {
                    continue;
                }

                match fs::read_dir(&year_path) {
                    Ok(month_entries) => {
                        for month_entry in month_entries {
                            let month_entry = match month_entry {
                                Ok(e) => e,
                                Err(e) => {
                                    errors.push(format!("Failed to read month dir: {}", e));
                                    continue;
                                }
                            };
                            let month_path = month_entry.path();
                            if !month_path.is_dir() {
                                continue;
                            }

                            match fs::read_dir(&month_path) {
                                Ok(day_entries) => {
                                    for day_entry in day_entries {
                                        let day_entry = match day_entry {
                                            Ok(e) => e,
                                            Err(e) => {
                                                errors.push(format!("Failed to read day entry: {}", e));
                                                continue;
                                            }
                                        };
                                        let path = day_entry.path();
                                        let file_name = path
                                            .file_name()
                                            .and_then(|n| n.to_str())
                                            .unwrap_or("");

                                        // Only match YYYY-MM-DD.md
                                        if !file_name.ends_with(".md") {
                                            continue;
                                        }
                                        let stem = &file_name[..file_name.len() - 3];
                                        if chrono::NaiveDate::parse_from_str(stem, "%Y-%m-%d").is_err() {
                                            continue;
                                        }

                                        // Check if .yaml already exists (idempotent)
                                        let yaml_path = month_path.join(format!("{}.yaml", stem));
                                        if yaml_path.exists() {
                                            skipped += 1;
                                            continue;
                                        }

                                        // Read .md, strip --- markers, write .yaml
                                        match convert_day_file(&path, &yaml_path) {
                                            Ok(()) => {
                                                converted += 1;
                                                // Delete old .md
                                                let _ = fs::remove_file(&path);
                                            }
                                            Err(e) => {
                                                errors.push(format!(
                                                    "Failed to convert {}: {}",
                                                    path.display(),
                                                    e
                                                ));
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    errors.push(format!(
                                        "Failed to read {}: {}",
                                        month_path.display(),
                                        e
                                    ));
                                }
                            }
                        }
                    }
                    Err(e) => {
                        errors.push(format!(
                            "Failed to read {}: {}",
                            year_path.display(),
                            e
                        ));
                    }
                }
            }
        }
        Err(e) => {
            return Err(format!("Failed to read root dir {}: {}", root.display(), e));
        }
    }

    // Bump version.txt to 2
    if converted > 0 || skipped > 0 {
        crate::files::write_version_file(root, 2)?;
    }

    eprintln!("Converted: {}, Skipped (already .yaml): {}", converted, skipped);
    if !errors.is_empty() {
        eprintln!("Errors:");
        for e in &errors {
            eprintln!("  - {}", e);
        }
    }

    Ok(())
}

fn convert_day_file(md_path: &Path, yaml_path: &Path) -> Result<(), String> {
    let content = fs::read_to_string(md_path)
        .map_err(|e| format!("Failed to read: {}", e))?;
    let content = content.trim_start_matches('\u{feff}');
    let content = content.trim();

    // Strip --- markers
    let yaml = if content.starts_with("---") {
        let after = &content[3..];
        if let Some(end) = after.find("\n---") {
            &after[..end]
        } else if after.ends_with("---") {
            &after[..after.len() - 3]
        } else {
            after
        }
    } else {
        content
    };

    let yaml = yaml.trim();
    if yaml.is_empty() {
        return Ok(());
    }

    // Validate it parses as DayFile
    let _: crate::models::DayFile = yaml_serde::from_str(yaml)
        .map_err(|e| format!("YAML parse error: {}", e))?;

    // Write with trailing newline
    fs::write(yaml_path, format!("{}\n", yaml))
        .map_err(|e| format!("Failed to write: {}", e))?;

    Ok(())
}
