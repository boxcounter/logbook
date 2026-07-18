use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// A guard that removes the PID lock file on drop.
#[derive(Debug)]
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
    /// Try to acquire the bundle-level single-instance lock.
    ///
    /// Writes the current PID to `{lock_dir}/instance.pid`. Kept for the
    /// per-bundle "no second instance of this app build" check; see
    /// `try_acquire_at` for the full semantics.
    pub fn try_acquire(lock_dir: &Path) -> Result<Self, InstanceLockError> {
        Self::try_acquire_at(&lock_dir.join("instance.pid"))
    }

    /// Try to acquire a lock at an explicit file path.
    ///
    /// Atomically creates the lock file (`create_new`, i.e. O_EXCL) containing
    /// the current PID, so two processes racing on an absent path cannot both
    /// win. If the file already exists:
    /// - names a live process → `Err(InstanceLockError::AlreadyRunning(pid))`.
    ///   A file naming *this* process is refused too: it means a leaked guard
    ///   in-process or a recycled PID, and refusing loudly beats silently
    ///   replacing a lock another holder believes it owns.
    /// - names a dead process / is unreadable / unparseable → the stale file
    ///   is removed and creation retried (bounded attempts: a competitor may
    ///   re-create the file between our remove and our create_new).
    pub fn try_acquire_at(lock_path: &Path) -> Result<Self, InstanceLockError> {
        if let Some(parent) = lock_path.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                // Not fatal by itself — the create_new below surfaces a
                // concrete Io error — but record it for diagnostics.
                crate::error_log::log_error(
                    "instance_lock",
                    &format!("Failed to create lock dir {}: {}", parent.display(), e),
                );
            }
        }

        for _ in 0..3 {
            match fs::OpenOptions::new().write(true).create_new(true).open(lock_path) {
                Ok(mut file) => {
                    file.write_all(format!("{}\n", std::process::id()).as_bytes())
                        .map_err(|e| {
                            crate::error_log::log_error(
                                "instance_lock",
                                &format!(
                                    "Failed to write lock file {}: {}",
                                    lock_path.display(),
                                    e
                                ),
                            );
                            InstanceLockError::Io(e)
                        })?;
                    return Ok(InstanceLock {
                        path: lock_path.to_path_buf(),
                    });
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    match existing_lock_holder(lock_path) {
                        Some(pid) => return Err(InstanceLockError::AlreadyRunning(pid)),
                        None => {
                            // Stale lock: remove and retry the atomic create.
                            let _ = fs::remove_file(lock_path);
                        }
                    }
                }
                Err(e) => {
                    crate::error_log::log_error(
                        "instance_lock",
                        &format!(
                            "Failed to create lock file {}: {}",
                            lock_path.display(),
                            e
                        ),
                    );
                    return Err(InstanceLockError::Io(e));
                }
            }
        }

        crate::error_log::log_error(
            "instance_lock",
            &format!(
                "Could not acquire lock {}: stale-lock replacement raced repeatedly",
                lock_path.display()
            ),
        );
        Err(InstanceLockError::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            "lock acquisition retries exhausted",
        )))
    }
}

/// Path of the cross-process writer lock for a data root:
/// `{root}/.logbook/writer.lock`.
///
/// The lock lives in the *data* directory itself so it mutually excludes
/// every process writing to the same root: the prod GUI, the dev GUI and the
/// CLI all derive the same path from the root. The bundle-level
/// `instance.pid` cannot do that — dev/prod use separate app-data dirs, and
/// the CLI has none of its own. `.logbook/` is the existing per-root
/// convention (operation logs live in `{root}/.logbook/operations/`).
pub fn writer_lock_path(root: &Path) -> PathBuf {
    root.join(".logbook").join("writer.lock")
}

/// Outcome of `swap_writer_lock`.
#[derive(Debug)]
pub enum WriterSwap {
    /// The lock for this root was already held; nothing changed.
    Kept,
    /// The new root's lock was acquired and the previous one released.
    Swapped,
}

/// Move a held data-root writer lock to `new_root`, releasing the previous
/// root's lock. Used when the GUI switches data roots at runtime
/// (`set_root_path`): the startup path only locks the root named by
/// root_path.txt, so without this the new root would stay unprotected until
/// the next launch (and the old root locked until exit).
///
/// Re-selecting the currently held root is a no-op (`WriterSwap::Kept`) —
/// important because `try_acquire_at` refuses even this process's own PID.
/// On `Err` the previous lock is retained untouched.
pub fn swap_writer_lock(
    held: &mut Option<(PathBuf, InstanceLock)>,
    new_root: &Path,
) -> Result<WriterSwap, InstanceLockError> {
    if let Some((root, _)) = held.as_ref() {
        if root == new_root {
            return Ok(WriterSwap::Kept);
        }
    }
    let guard = InstanceLock::try_acquire_at(&writer_lock_path(new_root))?;
    // Replacing the Option drops the old guard, which removes its lock file.
    *held = Some((new_root.to_path_buf(), guard));
    Ok(WriterSwap::Swapped)
}

/// PID recorded in an existing lock file if that process is still alive.
/// `None` means the file is stale (dead PID, unreadable, or unparseable) and
/// safe to replace.
fn existing_lock_holder(lock_path: &Path) -> Option<u32> {
    match read_lock_pid(lock_path) {
        Some(pid) if is_process_alive(pid) => Some(pid),
        Some(_) => None,
        None => {
            // Empty/unparseable: we may have caught the holder in the narrow
            // window between its atomic create and its PID write. Re-read once
            // after a short grace period before condemning the file as stale;
            // a genuinely orphaned empty file just costs one extra 50ms.
            std::thread::sleep(Duration::from_millis(50));
            match read_lock_pid(lock_path) {
                Some(pid) if is_process_alive(pid) => Some(pid),
                _ => None,
            }
        }
    }
}

fn read_lock_pid(lock_path: &Path) -> Option<u32> {
    fs::read_to_string(lock_path)
        .ok()?
        .trim()
        .parse::<u32>()
        .ok()
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

    fn temp_lock_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "logbook_lock_test_{}_{}",
            tag,
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn acquire_at_explicit_path_creates_and_cleans_up() {
        let dir = temp_lock_dir("explicit");
        let lock_path = dir.join("writer.lock");

        {
            let lock = InstanceLock::try_acquire_at(&lock_path).expect("acquire at explicit path");
            assert!(lock_path.exists());
            let content = std::fs::read_to_string(&lock_path).unwrap();
            assert_eq!(content.trim(), std::process::id().to_string());
            drop(lock);
        }

        assert!(!lock_path.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn acquire_at_creates_missing_parent_dirs() {
        let dir = temp_lock_dir("parents");
        let lock_path = dir.join(".logbook").join("writer.lock");

        let lock = InstanceLock::try_acquire_at(&lock_path)
            .expect("acquire should create missing .logbook dir");
        assert!(lock_path.exists());
        drop(lock);

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A lock held by a live process must block acquisition — even when the
    /// "live process" is this one (a second guard for the same path is a
    /// programming error, and PID reuse must not let a newcomer silently
    /// replace a lock). After the holder dies, the stale lock is replaced.
    #[cfg(unix)]
    #[test]
    fn live_holder_blocks_acquire_until_it_dies() {
        let dir = temp_lock_dir("live");
        let lock_path = dir.join("writer.lock");

        let mut child = std::process::Command::new("sleep")
            .arg("30")
            .spawn()
            .expect("spawn sleep");
        std::fs::write(&lock_path, format!("{}\n", child.id())).unwrap();

        match InstanceLock::try_acquire_at(&lock_path) {
            Err(InstanceLockError::AlreadyRunning(pid)) => assert_eq!(pid, child.id()),
            other => panic!("expected AlreadyRunning({}), got {:?}", child.id(), other),
        }

        child.kill().unwrap();
        child.wait().unwrap();

        let lock = InstanceLock::try_acquire_at(&lock_path)
            .expect("stale lock should be replaced after holder death");
        let content = std::fs::read_to_string(&lock_path).unwrap();
        assert_eq!(content.trim(), std::process::id().to_string());
        drop(lock);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn held_lock_blocks_same_process_reacquire() {
        let dir = temp_lock_dir("sameproc");
        let lock_path = dir.join("writer.lock");

        let lock1 = InstanceLock::try_acquire_at(&lock_path).expect("first acquire");
        match InstanceLock::try_acquire_at(&lock_path) {
            Err(InstanceLockError::AlreadyRunning(pid)) => {
                assert_eq!(pid, std::process::id())
            }
            other => panic!("expected AlreadyRunning(self), got {:?}", other),
        }
        drop(lock1);

        // After the guard is dropped (file removed), re-acquire succeeds.
        let lock2 = InstanceLock::try_acquire_at(&lock_path).expect("re-acquire after drop");
        drop(lock2);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn writer_lock_path_lives_under_data_root_dot_logbook() {
        let root = Path::new("/tmp/some-data-root");
        assert_eq!(
            writer_lock_path(root),
            PathBuf::from("/tmp/some-data-root/.logbook/writer.lock")
        );
    }

    /// O_EXCL creation must be atomic: with N threads racing to acquire the
    /// same path, exactly one wins and the rest get AlreadyRunning. Guards are
    /// collected before assertion so no winner drops its file mid-race.
    #[test]
    fn concurrent_acquire_exactly_one_winner() {
        use std::sync::{Arc, Barrier};

        let dir = temp_lock_dir("race");
        let lock_path = Arc::new(dir.join("writer.lock"));
        let barrier = Arc::new(Barrier::new(8));

        let handles: Vec<_> = (0..8)
            .map(|_| {
                let path = Arc::clone(&lock_path);
                let barrier = Arc::clone(&barrier);
                std::thread::spawn(move || {
                    barrier.wait();
                    InstanceLock::try_acquire_at(&path)
                })
            })
            .collect();

        let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

        let winners = results.iter().filter(|r| r.is_ok()).count();
        let blocked = results
            .iter()
            .filter(|r| matches!(r, Err(InstanceLockError::AlreadyRunning(_))))
            .count();
        assert_eq!(
            (winners, blocked),
            (1, 7),
            "exactly one thread may win the race: {:?}",
            results
        );

        drop(results);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn swap_writer_lock_acquires_new_root_and_releases_old() {
        let old_dir = temp_lock_dir("swap_old");
        let new_dir = temp_lock_dir("swap_new");
        let mut held = Some((
            old_dir.clone(),
            InstanceLock::try_acquire_at(&writer_lock_path(&old_dir)).unwrap(),
        ));

        let outcome = swap_writer_lock(&mut held, &new_dir).unwrap();

        assert!(matches!(outcome, WriterSwap::Swapped));
        assert_eq!(held.as_ref().unwrap().0, new_dir);
        assert!(!writer_lock_path(&old_dir).exists(), "old lock released");
        assert!(writer_lock_path(&new_dir).exists(), "new lock held");

        drop(held);
        let _ = std::fs::remove_dir_all(&old_dir);
        let _ = std::fs::remove_dir_all(&new_dir);
    }

    /// Re-selecting the root we already hold must not trip the
    /// same-process AlreadyRunning refusal.
    #[test]
    fn swap_writer_lock_same_root_is_kept() {
        let dir = temp_lock_dir("swap_same");
        let mut held = Some((
            dir.clone(),
            InstanceLock::try_acquire_at(&writer_lock_path(&dir)).unwrap(),
        ));

        let outcome = swap_writer_lock(&mut held, &dir).unwrap();

        assert!(matches!(outcome, WriterSwap::Kept));
        assert!(writer_lock_path(&dir).exists());

        drop(held);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A root locked by another holder refuses the swap, and the previously
    /// held lock is retained untouched.
    #[test]
    fn swap_writer_lock_locked_root_keeps_previous_lock() {
        let old_dir = temp_lock_dir("swap_keep");
        let busy_dir = temp_lock_dir("swap_busy");
        let mut held = Some((
            old_dir.clone(),
            InstanceLock::try_acquire_at(&writer_lock_path(&old_dir)).unwrap(),
        ));
        let _busy = InstanceLock::try_acquire_at(&writer_lock_path(&busy_dir)).unwrap();

        match swap_writer_lock(&mut held, &busy_dir) {
            Err(InstanceLockError::AlreadyRunning(_)) => {}
            other => panic!("expected AlreadyRunning, got {:?}", other),
        }
        assert_eq!(held.as_ref().unwrap().0, old_dir, "previous lock retained");
        assert!(writer_lock_path(&old_dir).exists());

        drop(held);
        drop(_busy);
        let _ = std::fs::remove_dir_all(&old_dir);
        let _ = std::fs::remove_dir_all(&busy_dir);
    }
}
