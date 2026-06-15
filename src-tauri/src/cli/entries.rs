use crate::cli::output;
use crate::files;
use std::path::Path;

pub fn list(root: &Path, date: &str, json: bool) {
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
