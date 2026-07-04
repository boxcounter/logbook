use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

/// Locate the logbook-cli binary and copy it to ~/.local/bin/.
///
/// Dev builds install as `logbook-cli-dev`; prod builds as `logbook-cli`.
/// Both can coexist without overwriting each other.
///
/// Search order:
///   1. Next to the current executable (dev: both binaries in target/debug/)
///   2. In the bundle's Resources directory (prod: bundled via tauri.conf.prod.json)
pub fn install_cli(resource_dir: Option<PathBuf>) -> Result<String, String> {
    // Source binary is always named "logbook-cli" (no suffix).
    let src_name = "logbook-cli";

    // Destination name varies by build profile so dev and prod can coexist.
    let dest_name = if env!("LOGBOOK_CLI_BUNDLE_ID").ends_with(".dev") {
        "logbook-cli-dev"
    } else {
        "logbook-cli"
    };

    let src = find_cli_binary(src_name, resource_dir)?;

    let home = std::env::var("HOME").map_err(|_| "HOME not set".to_string())?;
    let dest_dir = PathBuf::from(&home).join(".local").join("bin");
    std::fs::create_dir_all(&dest_dir)
        .map_err(|e| format!("Cannot create {}: {}", dest_dir.display(), e))?;

    let dest = dest_dir.join(dest_name);

    // Remove existing copy if present
    if dest.exists() {
        std::fs::remove_file(&dest)
            .map_err(|e| format!("Cannot remove existing {}: {}", dest.display(), e))?;
    }

    std::fs::copy(&src, &dest)
        .map_err(|e| format!("Cannot copy to {}: {}", dest.display(), e))?;

    let mut perms = std::fs::metadata(&dest)
        .map_err(|e| format!("Cannot read metadata: {}", e))?
        .permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&dest, perms)
        .map_err(|e| format!("Cannot set executable permission: {}", e))?;

    Ok(format!(
        "Installed {dest_name} → {dest}\n\n\
         Run: {dest_name} --help\n\
         Make sure {dir} is in your PATH. If not:\n  export PATH=\"$HOME/.local/bin:$PATH\"",
        dest_name = dest_name,
        dest = dest.display(),
        dir = dest_dir.display()
    ))
}

fn find_cli_binary(cli_name: &str, resource_dir: Option<PathBuf>) -> Result<PathBuf, String> {
    // 1. Look next to the current executable (covers dev and externalBin-bundled prod)
    let exe = std::env::current_exe().map_err(|e| format!("Cannot find executable: {}", e))?;
    let exe_dir = exe
        .parent()
        .ok_or_else(|| "Cannot determine executable directory".to_string())?;
    let sibling = exe_dir.join(cli_name);
    if sibling.exists() {
        return Ok(sibling);
    }

    // 2. Look in the bundle's Resources directory (resources-bundled prod)
    if let Some(res_dir) = resource_dir {
        let bundled = res_dir.join(cli_name);
        if bundled.exists() {
            return Ok(bundled);
        }
    }

    Err(format!(
        "CLI binary '{}' not found.\n\
         Checked: {}\n\
         If you are running from source, run:\n  cargo build --bin logbook-cli",
        cli_name,
        sibling.display()
    ))
}
