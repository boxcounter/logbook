use std::fs;
use std::path::{Path, PathBuf};

/// A guard that removes the PID lock file on drop.
pub struct InstanceLock {
    path: PathBuf,
}

impl InstanceLock {
    /// Try to acquire the single-instance lock.
    ///
    /// Writes the current PID to `{lock_dir}/instance.pid`. If the file already
    /// exists and its PID belongs to a live process, returns `Err(pid)`.
    /// Otherwise removes the stale file and creates a fresh one.
    pub fn try_acquire(lock_dir: &Path) -> Result<Self, u32> {
        let lock_path = lock_dir.join("instance.pid");

        if let Some(parent) = lock_path.parent() {
            fs::create_dir_all(parent).ok();
        }

        if let Ok(content) = fs::read_to_string(&lock_path) {
            if let Ok(pid) = content.trim().parse::<u32>() {
                if pid != std::process::id() && is_process_alive(pid) {
                    return Err(pid);
                }
                let _ = fs::remove_file(&lock_path);
            }
        }

        fs::write(&lock_path, format!("{}\n", std::process::id())).map_err(|_| 0u32)?;

        Ok(InstanceLock { path: lock_path })
    }
}

impl Drop for InstanceLock {
    fn drop(&mut self) {
        // Only remove if the file still contains our PID (avoid removing
        // a subsequent instance's lock that was created after us).
        if let Ok(content) = std::fs::read_to_string(&self.path) {
            if let Ok(pid) = content.trim().parse::<u32>() {
                if pid == std::process::id() {
                    let _ = fs::remove_file(&self.path);
                }
            }
        }
    }
}

fn is_process_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        std::process::Command::new("kill")
            .arg("-0")
            .arg(pid.to_string())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        false
    }
}
