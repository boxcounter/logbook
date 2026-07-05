# Menu Data Directory Actions — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add "Copy User Data Path" and "Open User Data Directory" menu items to the Logbook menu, remove the bottom day-file path bar from the UI.

**Architecture:** Rust `setup` hook handles everything — two new `MenuItem` instances in the Logbook submenu, click events processed directly in `on_menu_event` via `pbcopy` / `open`. Front-end simply deletes the path bar template and `useFileActions.ts` composable.

**Tech Stack:** Tauri 2.x (Rust menu API), macOS system commands (`pbcopy`, `open`), Vue 3.

## Global Constraints

- Menu items must use `MenuItemBuilder` with ids `copy-data-path` and `open-data-dir`, no keyboard shortcuts
- "Copy" feedback: menu text flips to "Copied!" for 1.5s, then restores; "Copy failed" on error + error_log
- "Open" feedback: none (macOS Finder handles directory-not-found natively); log error on spawn failure
- No root_path configured → error dialog "No data directory configured." (both items)
- `reveal_day_file` Tauri command must NOT be deleted
- `MenuItem` is `Send + Sync` in Tauri 2.x, safe for cross-thread `set_text` in 1.5s restore thread

---

### Task 1: Front-end cleanup — remove path bar and useFileActions.ts

**Files:**
- Modify: `src/components/MonthView.vue:8,58,226-233`
- Delete: `src/composables/useFileActions.ts`

**Interfaces:**
- Produces: clean MonthView.vue without path bar dependency

- [ ] **Step 1: Remove `useFileActions` import from MonthView.vue**

Open `src/components/MonthView.vue`. Delete line 8:

```
import { useFileActions } from "../composables/useFileActions";
```

Also delete the destructuring on line 58:

```
const { dayFilePath, displayPath, revealDayFile, copyFilePath, copiedFeedback } = useFileActions(store);
```

- [ ] **Step 2: Remove the path bar template block**

Delete lines 226–233 (the `<div v-if="store.rootPath" ...>` block) from MonthView.vue:

```html
      <div v-if="store.rootPath" class="mt-sm text-right flex justify-end items-baseline gap-md">
        <button
          class="text-micro text-[var(--color-text-disabled)] hover:text-[var(--color-text-secondary)] cursor-pointer"
          :title="store.rootPath + '/' + dayFilePath"
          @click="revealDayFile"
          @contextmenu.prevent="copyFilePath"
        >{{ copiedFeedback ? 'Copied!' : displayPath }}</button>
      </div>
```

- [ ] **Step 3: Delete useFileActions.ts**

```bash
rm src/composables/useFileActions.ts
```

- [ ] **Step 4: Verify front-end builds**

Run: `pnpm vue-tsc --noEmit`
Expected: no errors

- [ ] **Step 5: Commit front-end cleanup**

```bash
git add src/components/MonthView.vue src/composables/useFileActions.ts
git commit -m "refactor: remove bottom path bar and useFileActions composable"
```

---

### Task 2: Rust — add menu items and event handlers

**Files:**
- Modify: `src-tauri/src/lib.rs:12,96-164`

**Interfaces:**
- Consumes: clean MonthView.vue (Task 1)
- Produces: two new Logbook menu items with working click handlers

- [ ] **Step 1: Add required imports to lib.rs**

Insert after the existing `use std::path::PathBuf;` on line 12:

```rust
use std::io::Write;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;
```

- [ ] **Step 2: Add the two new MenuItem instances and update app_menu**

Replace lines 97–113 (from `let install_cli_item` through `.build()?;` of `app_menu`):

```rust
            let install_cli_item = MenuItemBuilder::new("Install Command Line Tool…")
                .id("install-cli")
                .build(app)?;

            let copy_data_path_item = MenuItemBuilder::new("Copy User Data Path")
                .id("copy-data-path")
                .build(app)?;

            let open_data_dir_item = MenuItemBuilder::new("Open User Data Directory")
                .id("open-data-dir")
                .build(app)?;

            let app_menu = SubmenuBuilder::new(app, "Logbook")
                .about(Some(Default::default()))
                .separator()
                .item(&install_cli_item)
                .separator()
                .item(&copy_data_path_item)
                .item(&open_data_dir_item)
                .separator()
                .services()
                .separator()
                .hide()
                .hide_others()
                .show_all()
                .separator()
                .quit()
                .build()?;
```

- [ ] **Step 3: Replace on_menu_event with move closure capturing the new items**

Replace lines 138–164 (the entire `app.on_menu_event(...)` block):

```rust
            let copy_item_for_event = copy_data_path_item.clone();
            let open_item_for_event = open_data_dir_item.clone();
            let app_data_dir_event = app_data_dir.clone();

            app.on_menu_event(move |app_handle, event| {
                match event.id().0 {
                    "install-cli" => {
                        crate::error_log::log_command_enter("install_cli", "menu");
                        let resource_dir = app_handle.path().resource_dir().ok();
                        match crate::cli::install::install_cli(resource_dir) {
                            Ok(msg) => {
                                crate::error_log::log_command_exit("install_cli", true, "");
                                let _ = app_handle
                                    .dialog()
                                    .message(msg)
                                    .title("Logbook")
                                    .kind(tauri_plugin_dialog::MessageDialogKind::Info)
                                    .show(|_| {});
                            }
                            Err(e) => {
                                crate::error_log::log_error("install_cli", &e);
                                crate::error_log::log_command_exit("install_cli", false, &e);
                                let _ = app_handle
                                    .dialog()
                                    .message(e)
                                    .title("Logbook — Install CLI Failed")
                                    .kind(tauri_plugin_dialog::MessageDialogKind::Error)
                                    .show(|_| {});
                            }
                        }
                    }
                    "copy-data-path" => {
                        let path = crate::files::read_root_path(&app_data_dir_event);
                        match path {
                            Some(p) => {
                                let path_str = p.to_string_lossy().to_string();
                                match Command::new("pbcopy")
                                    .stdin(Stdio::piped())
                                    .spawn()
                                {
                                    Ok(mut child) => {
                                        if child.stdin.as_mut().unwrap().write_all(path_str.as_bytes()).is_ok() {
                                            let _ = copy_item_for_event.set_text("Copied!");
                                        } else {
                                            crate::error_log::log_error(
                                                "copy-data-path",
                                                "pbcopy write failed",
                                            );
                                            let _ = copy_item_for_event.set_text("Copy failed");
                                        }
                                        let _ = child.wait();
                                    }
                                    Err(e) => {
                                        crate::error_log::log_error(
                                            "copy-data-path",
                                            &format!("pbcopy spawn failed: {}", e),
                                        );
                                        let _ = copy_item_for_event.set_text("Copy failed");
                                    }
                                }
                                let item = copy_item_for_event.clone();
                                thread::spawn(move || {
                                    thread::sleep(Duration::from_millis(1500));
                                    let _ = item.set_text("Copy User Data Path");
                                });
                            }
                            None => {
                                let _ = app_handle
                                    .dialog()
                                    .message("No data directory configured.")
                                    .title("Logbook")
                                    .kind(tauri_plugin_dialog::MessageDialogKind::Error)
                                    .show(|_| {});
                            }
                        }
                    }
                    "open-data-dir" => {
                        let path = crate::files::read_root_path(&app_data_dir_event);
                        match path {
                            Some(p) => {
                                match Command::new("open").arg(&p).spawn() {
                                    Ok(_) => {}
                                    Err(e) => {
                                        crate::error_log::log_error(
                                            "open-data-dir",
                                            &format!("open failed: {}", e),
                                        );
                                    }
                                }
                            }
                            None => {
                                let _ = app_handle
                                    .dialog()
                                    .message("No data directory configured.")
                                    .title("Logbook")
                                    .kind(tauri_plugin_dialog::MessageDialogKind::Error)
                                    .show(|_| {});
                            }
                        }
                    }
                    _ => {}
                }
            });
```

- [ ] **Step 4: Verify Rust compiles**

Run: `cd src-tauri && cargo check`
Expected: no errors

- [ ] **Step 5: Run all tests**

Run: `pnpm test`
Expected: all tests pass

- [ ] **Step 6: Commit Rust changes**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat: add Copy User Data Path and Open User Data Directory menu items"
```
