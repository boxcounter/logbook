/// Contract test runner (TEST PHASE — intentionally non-compiling / panicking stubs).
///
/// Reads YAML contract files from `tests/contracts/`, sets up temp fixture dirs,
/// dispatches to real Rust functions, and asserts results against expectations.
///
/// These tests will fail to compile (or panic at runtime via `unimplemented!()`)
/// because the runner infrastructure hasn't been built yet. This file is the
/// test-first step of the implementation cycle.

// STUB: will be replaced by the real contract runner implementation.
fn run_contract(_yaml_path: &str) {
    unimplemented!("contract runner not yet implemented");
}

#[test]
fn contract_get_entries() {
    run_contract("contracts/get_entries.yaml");
}

#[test]
fn contract_append_entry() {
    run_contract("contracts/append_entry.yaml");
}
