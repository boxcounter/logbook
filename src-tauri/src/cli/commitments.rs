use crate::cli::output;
use crate::files;
use crate::models::{Commitment, CommitmentProgress, MonthlyFile};
use std::io::Read;
use std::path::Path;

pub fn list(root: &Path, year: i32, month: u32, json: bool) {
    let monthly = files::read_monthly_file(root, year, month).unwrap_or_else(|e| {
        output::print_error(&format!("Failed to read monthly file: {}", e));
        std::process::exit(1);
    });

    output::print_output(
        json,
        &monthly.commitments,
        &format_commitments_human(&monthly.commitments),
    );
}

pub fn progress(root: &Path, year: i32, month: u32, json: bool) {
    let prog = crate::commands::get_commitment_progress(
        root.to_string_lossy().into_owned(),
        year,
        month,
    )
    .unwrap_or_else(|e| {
        output::print_error(&format!("Failed to get commitment progress: {}", e));
        std::process::exit(1);
    });

    output::print_output(
        json,
        &prog,
        &format_progress_human(&prog),
    );
}

pub fn set(root: &Path, year: i32, month: u32, json: bool) {
    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .unwrap_or_else(|e| {
            output::print_error(&format!("Failed to read stdin: {}", e));
            std::process::exit(1);
        });

    // Try JSON first, then YAML
    let monthly: MonthlyFile =
        if let Ok(mf) = serde_json::from_str::<MonthlyFile>(&input) {
            mf
        } else if let Ok(mf) = yaml_serde::from_str::<MonthlyFile>(&input) {
            mf
        } else {
            output::print_error(
                "Failed to parse input as JSON or YAML MonthlyFile.\n\
                 Expected JSON: {\"commitments\":[{\"role\":\"...\",\"allocation\":N,\"goals\":[...]}]}\n\
                 Or YAML:\ncommitments:\n  - role: Dev\n    allocation: 40\n    goals:\n      - Goal name",
            );
            std::process::exit(1);
        };

    // Route through the same command the GUI uses: validates, applies goal renames,
    // protects goals referenced by entries, and preserves the month's dimensions block.
    // Dimensions are file-managed (not set via this command); any in the input are ignored.
    match crate::commands::set_commitments(
        root.to_string_lossy().into_owned(),
        year,
        month,
        monthly.commitments,
    ) {
        Ok(_) => output::print_output(
            json,
            &serde_json::json!({"ok": true}),
            "Commitments written successfully.",
        ),
        Err(e) => {
            output::print_error(&e);
            std::process::exit(1);
        }
    }
}

fn format_commitments_human(commitments: &[Commitment]) -> String {
    if commitments.is_empty() {
        return "(no commitments)".to_string();
    }
    let mut out = String::new();
    for c in commitments {
        out.push_str(&format!("Role: {} ({}h/month)\n", c.role, c.allocation));
        for g in &c.goals {
            out.push_str(&format!("  - {}\n", g));
        }
        out.push('\n');
    }
    out.trim_end().to_string()
}

fn format_progress_human(progress: &[CommitmentProgress]) -> String {
    if progress.is_empty() {
        return "(no commitments)".to_string();
    }
    let mut out = String::new();
    for c in progress {
        let pct = if c.allocation_minutes > 0 {
            (c.spent_minutes as f64 / c.allocation_minutes as f64) * 100.0
        } else {
            0.0
        };
        out.push_str(&format!(
            "Role: {} ({:.0}% — {:.1}h / {}h)\n",
            c.role,
            pct,
            c.spent_minutes as f64 / 60.0,
            c.allocation_minutes / 60
        ));
        for g in &c.goals {
            out.push_str(&format!(
                "  - {}: {:.1}h\n",
                g.name,
                g.spent_minutes as f64 / 60.0
            ));
        }
        out.push('\n');
    }
    out.trim_end().to_string()
}
