use crate::models::{IntegrityIssue, IntegrityStatus};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, Mutex};

static INTEGRITY_OK: LazyLock<AtomicBool> = LazyLock::new(|| AtomicBool::new(true));
static INTEGRITY_ISSUES: LazyLock<Mutex<Vec<IntegrityIssue>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

pub fn check() -> Result<(), String> {
    if INTEGRITY_OK.load(Ordering::Acquire) {
        Ok(())
    } else {
        let issues = INTEGRITY_ISSUES
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let msg = if issues.is_empty() {
            "Write denied: data integrity compromised".to_string()
        } else {
            format!(
                "Write denied: data integrity compromised ({} issue{})",
                issues.len(),
                if issues.len() == 1 { "" } else { "s" }
            )
        };
        Err(msg)
    }
}

pub fn set_compromised(issue: IntegrityIssue) {
    INTEGRITY_ISSUES
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .push(issue);
    INTEGRITY_OK.store(false, Ordering::Release);
}

pub fn reset() {
    INTEGRITY_ISSUES
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clear();
    INTEGRITY_OK.store(true, Ordering::Release);
}

pub fn status() -> IntegrityStatus {
    let ok = INTEGRITY_OK.load(Ordering::Acquire);
    let issues = INTEGRITY_ISSUES
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    IntegrityStatus {
        compromised: !ok,
        issues,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_uncompromised() {
        reset();
        assert!(check().is_ok());
        let s = status();
        assert!(!s.compromised);
        assert!(s.issues.is_empty());
    }

    #[test]
    fn set_compromised_blocks_writes() {
        reset();
        set_compromised(IntegrityIssue {
            path: "2026/07/05.md".into(),
            message: "corrupt YAML".into(),
            kind: "YamlParseError".into(),
        });
        assert!(check().is_err());
        let s = status();
        assert!(s.compromised);
        assert_eq!(s.issues.len(), 1);
        assert_eq!(s.issues[0].kind, "YamlParseError");
    }

    #[test]
    fn reset_restores_writes() {
        set_compromised(IntegrityIssue {
            path: "x.md".into(),
            message: "bad".into(),
            kind: "Test".into(),
        });
        reset();
        assert!(check().is_ok());
        let s = status();
        assert!(!s.compromised);
        assert!(s.issues.is_empty());
    }

    #[test]
    fn multiple_issues_accumulate() {
        reset();
        set_compromised(IntegrityIssue {
            path: "a.md".into(),
            message: "e1".into(),
            kind: "K1".into(),
        });
        set_compromised(IntegrityIssue {
            path: "b.md".into(),
            message: "e2".into(),
            kind: "K2".into(),
        });
        assert!(check().is_err());
        let s = status();
        assert_eq!(s.issues.len(), 2);
    }
}
