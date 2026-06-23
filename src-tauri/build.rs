fn main() {
    // Bundle ID mirrors Tauri's config selection:
    //   debug   → tauri.conf.json     → com.boxcounter.logbook.dev
    //   release → tauri.conf.prod.json → com.boxcounter.logbook
    let bundle_id = if cfg!(debug_assertions) {
        "com.boxcounter.logbook.dev"
    } else {
        "com.boxcounter.logbook"
    };
    println!("cargo:rustc-env=LOGBOOK_CLI_BUNDLE_ID={}", bundle_id);

    tauri_build::build()
}
