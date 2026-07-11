use crate::cli::output;
use crate::files;
use std::path::Path;

pub fn list(root: &Path, date: &str, json: bool) {
    if let Err(e) = crate::commands::validate_date_format(date) {
        output::print_error(&e);
        std::process::exit(1);
    }
    let day_file = files::read_day_file(root, date).unwrap_or_else(|e| {
        output::print_error(&format!("Failed to read day file: {}", e));
        std::process::exit(1);
    });

    output::print_output(
        json,
        &day_file,
        &format_entries_human(&day_file, date),
    );
}

pub fn add(root: &Path, date: &str, json: bool) {
    if let Err(e) = crate::integrity::check() {
        output::print_error(&e);
        std::process::exit(1);
    }

    use std::io::Read;

    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .unwrap_or_else(|e| {
            output::print_error(&format!("Failed to read stdin: {}", e));
            std::process::exit(1);
        });

    let entry_input: crate::models::CreateEntryInput =
        serde_json::from_str(&input).unwrap_or_else(|e| {
            output::print_error(&format!(
                "Failed to parse input as CreateEntryInput JSON.\n\
                 Expected: {{\"item\":\"...\",\"duration\":\"...\",\"dimensions\":{{...}}}}\n\
                 Error: {}",
                e
            ));
            std::process::exit(1);
        });

    let entry = crate::commands::append_entry(
        root.to_string_lossy().into_owned(),
        date.to_string(),
        entry_input,
    )
    .unwrap_or_else(|e| {
        output::print_error(&e);
        std::process::exit(1);
    });

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&entry).expect("Failed to serialize entry")
        );
    } else {
        let dims: Vec<String> = entry
            .dimensions
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();
        println!(
            "Added: \"{}\" | {}m | {}",
            entry.item,
            entry.duration,
            dims.join(", ")
        );
    }
}

fn format_entries_human(day_file: &crate::models::DayFile, date: &str) -> String {
    if day_file.entries.is_empty() && day_file.note.is_none() {
        return format!("{}: (no entries)", date);
    }
    let mut out = format!("=== {} ===\n", date);
    if let Some(ref note) = day_file.note {
        out.push_str(&format!("Note: {}\n\n", note));
    }
    if day_file.entries.is_empty() {
        out.push_str("(no entries)\n");
        return out;
    }
    for e in &day_file.entries {
        let dims: Vec<String> = e
            .dimensions
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();
        out.push_str(&format!(
            "  {} | {}m | {}\n",
            e.item,
            e.duration,
            dims.join(", ")
        ));
    }
    let total: u32 = day_file.entries.iter().map(|e| e.duration).sum();
    out.push_str(&format!("  ---\n  Total: {}m ({:.1}h)\n", total, total as f64 / 60.0));
    out
}

pub fn update(root: &Path, date: &str, entry_id: &str, json: bool) {
    if let Err(e) = crate::integrity::check() {
        output::print_error(&e);
        std::process::exit(1);
    }

    use std::io::Read;

    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .unwrap_or_else(|e| {
            output::print_error(&format!("Failed to read stdin: {}", e));
            std::process::exit(1);
        });

    let update_input: crate::models::UpdateEntryInput =
        serde_json::from_str(&input).unwrap_or_else(|e| {
            output::print_error(&format!(
                "Failed to parse input as UpdateEntryInput JSON.\n\
                 Expected: {{\"item\":\"...\",\"duration\":\"...\",\"dimensions\":{{...}}}}\n\
                 Error: {}",
                e
            ));
            std::process::exit(1);
        });

    let day_file = crate::commands::update_entry(
        root.to_string_lossy().into_owned(),
        date.to_string(),
        entry_id.to_string(),
        update_input,
    )
    .unwrap_or_else(|e| {
        output::print_error(&e);
        std::process::exit(1);
    });

    // Extract the single updated entry from the returned DayFile.
    // update_entry succeeds only if the entry exists, so find() cannot fail.
    let entry = day_file
        .entries
        .iter()
        .find(|e| e.id == entry_id)
        .unwrap_or_else(|| {
            output::print_error("Updated entry not found in result");
            std::process::exit(1);
        });

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(entry).expect("Failed to serialize entry")
        );
    } else {
        let dims: Vec<String> = entry
            .dimensions
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();
        println!(
            "Updated: \"{}\" | {}m | {}",
            entry.item,
            entry.duration,
            dims.join(", ")
        );
    }
}

pub fn delete(root: &Path, date: &str, entry_id: &str, json: bool) {
    if let Err(e) = crate::integrity::check() {
        output::print_error(&e);
        std::process::exit(1);
    }

    // The backend returns the remaining DayFile, but the deleted entry no longer
    // exists — we only emit a confirmation, not entry data.
    crate::commands::delete_entry(
        root.to_string_lossy().into_owned(),
        date.to_string(),
        entry_id.to_string(),
    )
    .unwrap_or_else(|e| {
        output::print_error(&e);
        std::process::exit(1);
    });

    if json {
        let confirmation = serde_json::json!({"ok": true, "date": date, "entry_id": entry_id});
        println!(
            "{}",
            serde_json::to_string_pretty(&confirmation).expect("Failed to serialize confirmation")
        );
    } else {
        println!("Deleted: {} from {}", entry_id, date);
    }
}
