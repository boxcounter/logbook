use serde::Serialize;

/// Print `data` as either JSON (pretty) or a human-readable string.
/// If `json` is true, print JSON. Otherwise, print `human` string.
pub fn print_output<T: Serialize>(json: bool, data: &T, human: &str) {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(data).expect("Failed to serialize output")
        );
    } else {
        println!("{}", human);
    }
}

/// Print error to stderr.
pub fn print_error(msg: &str) {
    eprintln!("Error: {}", msg);
}
