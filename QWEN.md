# CLAUDE.md — Basie-64

Offline-first Base64 encoder/decoder built in Rust + egui (eframe). Goal: *the* Base64 tool a developer reaches for — best-in-class at a narrow job, never phones home.

**Active roadmap:** `POLISH_PLAN.md`. Current milestone: **v0.3 — "Feels Finished"** (Phase 1 complete).

---

## Module map

```
src/
├── main.rs        Entry point: window setup, theme bootstrap, run_native.
├── cli.rs         CLI binary entry point (clap) — Phase 2.
├── app.rs         Basie64App state + eframe::App::update dispatcher. Holds all mutable UI state.
├── theme.rs       Theme enum (Light/Dark/System), palette application, icon loading.
├── settings.rs    Persisted prefs (theme, shortcut-hint flag, recent files, private_mode) — TOML in OS config dir via `directories`.
├── decode.rs      decode_input_str (impl on Basie64App) + DecodeHint / infer_hint for actionable errors.
├── detect.rs      Smart-detection scan (regex + mixed-content). Reads/writes app state in place.
├── samples.rs     Hard-coded sample payloads (JWT, PNG data URI, JSON) for the Samples menu.
├── core/
│   ├── mod.rs     Re-exports for pure-logic modules.
│   └── history.rs HistoryEntry + HistoryStore (JSON persistence, FIFO eviction, stable IDs, exact delete, search, full-input reload). Phase 2.
└── ui/
    ├── mod.rs       Module exports.
    ├── top_bar.rs   Draggable titlebar, theme toggle, private mode settings toggle, history-panel toggle, close button.
    ├── input.rs     Input text area, empty-state hint, samples menu, shortcut-hint row.
    ├── buttons.rs   Action row: Encode / Decode / Save as File / Clear. Handles large-paste confirm.
    ├── output.rs    Output text area (monospace), Copy / Copy as Data URI, image preview, copy-pulse.
    ├── banner.rs    Smart-detection banner (with fade-in), mixed-matches list, error + hint row.
    └── history_panel.rs  Collapsible bottom panel: dedicated search, selection, Enter/double-click reload, per-entry delete, clear all.
```

`Basie64App` fields are `pub(crate)` — UI modules take `&mut Basie64App` and read/write directly. No event bus, no `Rc<RefCell>`.

---

## Build & test

```sh
cargo run                         # launch the app
cargo test                        # unit tests
cargo fmt                         # format
cargo clippy --all-targets -- -D warnings   # lint (must be clean)
```

---

## Architectural rules

- **`core/` modules must have zero `egui` imports.** Pure Rust only — they're shared with the future CLI companion.
- **`decode.rs` must stay UI-free on its pure helpers** (`infer_hint`, future `decode` free function). The Phase 2 plan ships a CLI companion reusing this core — don't entangle it with `egui::Context` beyond what's already there.
- **No `unwrap` / `expect` on user-input paths.** Safe exceptions: compile-time-static regex (`Basie64App::default`), and the static regex in `detect::tests`. Everything else should return `Option`/`Result` and fail gracefully.
- **All persisted state goes through `settings::Settings` or a dedicated store in `core/`.** Don't scatter file reads across modules. `Settings::save()` and `HistoryStore::save()` are fire-and-forget (ignore I/O errors by design — we never crash the UI on disk hiccups).
- **Theme changes must go through `theme::apply`.** Don't mutate `ctx.style()` ad-hoc from UI code.
- **Don't add telemetry, crash reporting, or network calls** without making them opt-in and clearly scoped. The privacy pitch is load-bearing for this project.

---

## Where things live (quick lookups)

| Want to change... | Edit |
|---|---|
| Color palette / spacing | `theme.rs` |
| A keyboard shortcut | `app.rs` (`update` → `ctx.input`) |
| Button row layout | `ui/buttons.rs` |
| Sample payloads menu | `samples.rs` |
| Decode logic (JWT, variants, data URI) | `decode.rs` |
| Smart-detection regex / scan | `detect.rs` |
| Error messages and hints | `decode.rs::DecodeHint` + `ui/banner.rs::show_error` |
| Config file format | `settings.rs` |
| History persistence & data model | `core/history.rs` |
| History panel UI | `ui/history_panel.rs` |

---

## Explicit non-goals

Copied from `POLISH_PLAN.md`:

- ❌ Cloud sync, accounts, telemetry by default
- ❌ Becoming a general encoding/hashing/crypto toolkit
- ❌ Mobile ports
- ❌ Monetization / paid tiers
- ❌ Web version (defeats the offline-first pitch)

Before adding a feature, check it isn't on this list.
