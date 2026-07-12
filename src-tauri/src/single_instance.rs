use std::fs;
use std::path::{Path, PathBuf};

/// A guard that removes the PID lock file on drop.
pub struct InstanceLock {
    path: PathBuf,
}

/// Errors that can occur when acquiring the instance lock.
#[derive(Debug)]
pub enum InstanceLockError {
    /// Another instance is running with this PID.
    AlreadyRunning(u32),
    /// Cannot read/write lock file (permission, disk full, etc.).
    Io(std::io::Error),
}

impl InstanceLock {
    /// Try to acquire the single-instance lock.
    ///
    /// Writes the current PID to `{lock_dir}/instance.pid`. If the file already
    /// exists and its PID belongs to a live process, returns
    /// `Err(InstanceLockError::AlreadyRunning(pid))`.
    /// Otherwise removes the stale file and creates a fresh one.
    pub fn try_acquire(lock_dir: &Path) -> Result<Self, InstanceLockError> {
        let lock_path = lock_dir.join("instance.pid");

        if let Some(parent) = lock_path.parent() {
            fs::create_dir_all(parent).ok();
        }

        if let Ok(content) = fs::read_to_string(&lock_path) {
            if let Ok(pid) = content.trim().parse::<u32>() {
                if pid != std::process::id() && is_process_alive(pid) {
                    return Err(InstanceLockError::AlreadyRunning(pid));
                }
                let _ = fs::remove_file(&lock_path);
            }
        }

        fs::write(&lock_path, format!("{}\n", std::process::id())).map_err(|e| {
            crate::error_log::log_error("instance_lock",
                &format!("Failed to write lock file {}: {}", lock_path.display(), e));
            InstanceLockError::Io(e)
        })?;

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
        crate::error_log::log_error(
            "single_instance",
            &format!(
                "is_process_alive not implemented on this platform — single-instance enforcement is disabled. PID check requested for {}",
                pid
            ),
        );
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acquire_and_drop_cleans_up() {
        let dir = std::env::temp_dir().join(format!("logbook_lock_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();

        {
            let lock = InstanceLock::try_acquire(&dir).expect("first acquire should succeed");
            assert!(dir.join("instance.pid").exists());
            drop(lock);
        }

        assert!(!dir.join("instance.pid").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn stale_lock_is_replaced() {
        let dir = std::env::temp_dir().join(format!("logbook_lock_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();

        let lock_path = dir.join("instance.pid");
        std::fs::write(&lock_path, "99999\n").unwrap();

        let lock = InstanceLock::try_acquire(&dir).expect("should acquire after stale PID");
        let content = std::fs::read_to_string(&lock_path).unwrap();
        assert_eq!(content.trim(), std::process::id().to_string());
        drop(lock);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn same_pid_reacquires_lock() {
        let dir = std::env::temp_dir().join(format!("logbook_lock_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();

        let lock1 = InstanceLock::try_acquire(&dir).expect("first acquire");
        drop(lock1);

        let lock2 = InstanceLock::try_acquire(&dir).expect("second acquire after drop");
        drop(lock2);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn current_process_is_alive() {
        assert!(is_process_alive(std::process::id()));
    }

    #[test]
    fn invalid_pid_is_dead() {
        assert!(!is_process_alive(u32::MAX));
    }
}
