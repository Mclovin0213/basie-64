# CLAUDE.md — Basie-64

Offline-first Base64 encoder/decoder built in Rust + egui (eframe), shipped as both a GUI (`basie-64`) and a CLI (`basie`). Goal: *the* Base64 tool a developer reaches for — best-in-class at a narrow job, never phones home.

**Active roadmap:** `POLISH_PLAN.md`. Phase 2 ("Power User") is shipped — including the richer image flow (EXIF extraction, metadata bar, Export Image dialog with lossless metadata stripping). Phase 3+ (code-quality audit, branding, packaging, distribution, launch) is still pending.

---

## Module map

```
src/
├── main.rs              GUI entry: window setup, theme bootstrap, run_native.
├── lib.rs               Library root — exposes core/ so the CLI and tests can link.
├── cli.rs               CLI entry: clap subcommands (encode/decode/convert/detect/diff/hash), stdin + file I/O.
├── app.rs               Basie64App state + eframe::App::update dispatcher. Keyboard shortcuts, drag-drop, feature toggles.
├── theme.rs             Theme enum (Light/Dark/System), palette application, icon loading.
├── settings.rs          Persisted prefs (theme, private_mode, recent files) — TOML in OS config dir via `directories`.
├── samples.rs           Hard-coded sample payloads (JWT, PNG data URI, JSON) for the Samples menu.
├── decode.rs            Thin app-level wrapper: calls core::decode and mutates Basie64App state (output, jwt_inspection, image_preview, image_meta, error, history).
├── detect.rs            Thin app-level wrapper: calls core::detect and mutates Basie64App state (detected_format, banners, mixed_matches, diff split).
├── core/                Pure Rust. Zero egui imports. Shared by GUI and CLI.
│   ├── mod.rs           Submodule declarations.
│   ├── encode.rs        encode_base64 / encode_base64_bytes.
│   ├── decode.rs        decode_base64 → DecodeOutput { Jwt | Text | Binary }. Handles data URIs and three variant fallbacks (STANDARD / URL_SAFE / URL_SAFE_NO_PAD). Pretty-prints JSON.
│   ├── detect.rs        Format detection with priority Percent → Hex → Base32 → Base58 → Base64. Returns DetectionResult + banner text + mixed-content matches + optional diff split on `\n---\n`.
│   ├── jwt.rs           Structured JWT inspection: header/payload parse, RFC 7519 claim explanations, humanized exp/iat/nbf, warnings (alg:none, expired, nbf in future), HMAC verification (HS256/384/512).
│   ├── hash.rs          sha256_hex, md5_hex, sha256_base64.
│   ├── history.rs       HistoryEntry + HistoryStore. TOML persistence, FIFO eviction at 200 entries, stable IDs, search, exact delete, full-input reload.
│   ├── diff.rs          parse_diff_input (splits on `\n---\n` or `\n===\n`), diff_text via `similar` crate, diff_binary hex-dump.
│   ├── batch.rs         BatchOp / BatchSource / BatchConfig / BatchPreview / BatchProgress / BatchResult. process_batch_with_progress runs the threaded pipeline.
│   ├── convert.rs       Cross-format conversion between Base64 / Hex / Base32 / Base58 / PercentEncoded.
│   ├── image_meta.rs    Image kind detection (PNG/JPEG/GIF/WebP/BMP/ICO), dimensions, EXIF parsing via `kamadak-exif`, lossless metadata stripping (EXIF / text chunks / XMP / IPTC).
│   └── command_registry.rs  Static Command list (id, name, keywords, shortcut) + filter_commands fuzzy search for the palette.
└── ui/                  egui widgets. Imports core/ for logic and reads/writes Basie64App state directly.
    ├── mod.rs
    ├── top_bar.rs       Draggable titlebar, theme toggle, settings menu (private-mode toggle), history-panel toggle, close button.
    ├── input.rs         Input text area, empty-state hint, samples menu, large-paste guard.
    ├── buttons.rs       Action row: Encode / Decode / Diff / Save as File / Clear + batch folder/file dialogs.
    ├── output.rs        Output monospace area, Copy / Copy as Data URI, image preview + image metadata bar (kind/dimensions/size, EXIF collapsible, Export… button), JWT inspector subpanel (payload viewer + HMAC secret input + verification).
    ├── export_image_dialog.rs  Modal dialog for saving decoded images with optional lossless metadata stripping; backdrop + centered window, EXIF field list, strip checkbox, Save / Cancel.
    ├── banner.rs        Smart-detection banner (with fade-in), convert-format hint, mixed-matches list, error + hint row.
    ├── history_panel.rs Bottom panel: search box, entry list, Enter/double-click reload, Delete removes, Clear All.
    ├── batch_panel.rs   Bottom panel: batch preview, confirmation, progress, results table.
    ├── diff_view.rs     Full-screen diff mode: side-by-side text/binary comparison + summary stats.
    └── command_palette.rs  Cmd+K overlay: fuzzy search, arrow/enter/escape, dispatches to Basie64App methods.
```

`Basie64App` fields are `pub(crate)` — UI modules take `&mut Basie64App` and read/write directly. No event bus, no `Rc<RefCell>`.

---

## Build & test

```sh
cargo run                                    # launch the GUI
cargo run --bin basie -- encode "Hello"      # run the CLI
cargo test                                   # unit tests across core/
cargo fmt                                    # format
cargo clippy --all-targets -- -D warnings    # lint (must be clean)
```

---

## Architectural rules

- **`core/` modules must have zero `egui` imports.** This boundary is load-bearing — `src/cli.rs` links only against `core/`, so any accidental `use egui::…` inside `core/` breaks the CLI build. Pure Rust only.
- **`src/decode.rs` and `src/detect.rs` are app-state adapters**, not logic. They call into `core::decode` / `core::detect`, then update `Basie64App` (output, banners, history, error). Keep the pure helpers in `core/` and the side effects out here.
- **No `unwrap` / `expect` on user-input paths.** Safe exceptions: compile-time-static regex, test-only assertions. Everything else returns `Option` / `Result` and fails gracefully — the UI should never crash on a bad paste.
- **All persisted state goes through `settings::Settings` or a dedicated store in `core/` (e.g. `HistoryStore`).** Don't scatter file reads across modules. Saves are fire-and-forget — we never crash the UI on disk hiccups.
- **Theme changes must go through `theme::apply`.** Don't mutate `ctx.style()` ad-hoc from UI code.
- **Don't add telemetry, crash reporting, or network calls** without making them opt-in and clearly scoped. The privacy pitch is load-bearing for this project.

---

## Where things live (quick lookups)

| Want to change... | Edit |
|---|---|
| Color palette / spacing | `src/theme.rs` |
| A keyboard shortcut | `src/app.rs` (`update` → `ctx.input`) |
| Button row layout | `src/ui/buttons.rs` |
| Sample payloads menu | `src/samples.rs` + `src/ui/input.rs` |
| Base64 encode/decode logic | `src/core/encode.rs`, `src/core/decode.rs` |
| JWT parsing / warnings / HMAC verification | `src/core/jwt.rs` + `src/ui/output.rs` (inspector card) |
| Smart-detection regex / scan | `src/core/detect.rs` |
| Multi-format conversion (Hex/Base32/Base58/Percent) | `src/core/convert.rs` |
| Hash functions | `src/core/hash.rs` |
| History persistence & data model | `src/core/history.rs` |
| History panel UI | `src/ui/history_panel.rs` |
| Batch processing pipeline | `src/core/batch.rs` |
| Batch UI (preview, progress, results) | `src/ui/batch_panel.rs` |
| Diff algorithm (text + hex-dump) | `src/core/diff.rs` |
| Diff view UI | `src/ui/diff_view.rs` |
| Command palette entries / fuzzy match | `src/core/command_registry.rs` |
| Command palette overlay UI | `src/ui/command_palette.rs` |
| Image kind / dimensions / EXIF / metadata stripping | `src/core/image_meta.rs` |
| Image preview + metadata bar UI | `src/ui/output.rs` (`image_meta_bar` submodule) |
| Export Image dialog (lossless strip) | `src/ui/export_image_dialog.rs` |
| Error messages and hints | `src/core/decode.rs` + `src/ui/banner.rs` |
| Config file format | `src/settings.rs` |
| CLI subcommands | `src/cli.rs` |

---

## Explicit non-goals

See `POLISH_PLAN.md` § "Explicit Non-Goals". Before adding a feature, check it isn't on that list.
