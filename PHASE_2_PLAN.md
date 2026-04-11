# Phase 2 Implementation Plan — "Power User"

Detailed breakdown of Phase 2 features from `POLISH_PLAN.md`. Goal: things competing web tools *can't* easily do, that make Basie-64 the obvious pick.

---

## 1. History Panel

**What:** A timestamped, searchable list of recent encode/decode operations. Local only, clearable, with a "private mode" toggle.

**Data model** (`history.rs`):
- Each entry: `{timestamp, operation: Encode|Decode, input_preview, output_preview, variant}`
- Stored in a separate JSON file in the OS config dir (keeps `config.toml` lean)
- Max 200 entries by default, FIFO eviction

**UI** (`ui/history_panel.rs`):
- Collapsible sidebar or bottom panel
- Search bar at top (filters on input/output preview)
- Each row shows: operation icon, timestamp, truncated input → output
- Double-click or Enter reloads the entry into the input field
- "Clear all" button + per-entry delete (Delete key or ✕ button)
- "Private mode" toggle in settings — when active, nothing is logged

**Integration with Settings:**
- `Settings` gains `history_path: Option<PathBuf>` or a dedicated `HistoryStore` struct
- `HistoryStore::append()`, `HistoryStore::search()`, `HistoryStore::clear()`

**Tests:**
- Append + eviction (FIFO at max capacity)
- Search filtering (prefix + substring)
- Private mode bypass (append is a no-op)
- Round-trip: load entry back into input and decode

---

## 2. Batch Mode

**What:** Drop a folder (or select multiple files), encode/decode every file, export results as a manifest.

**Logic** (`batch.rs`):
- Traverse directory recursively (optional depth limit)
- For encode: read each file → Base64 → write `.b64` sibling or output dir
- For decode: read each `.b64` file → decode → write decoded sibling or output dir
- Track per-file status: success, skipped, error (with reason)
- Return a `BatchResult { processed, succeeded, failed, output_dir }`

**UI:**
- Drop zone already accepts single files — extend to accept folders
- When folder dropped, show a batch confirmation dialog
- Results displayed in a scrollable table: file name, status icon, size, error message
- "Export manifest" button → writes JSON listing all files, their status, and output paths
- Progress indicator for large batches

**Manifest format:**
```json
{
  "version": 1,
  "timestamp": "2026-04-11T12:00:00Z",
  "operation": "encode",
  "files": [
    { "input": "photo.png", "output": "photo.png.b64", "status": "ok", "input_size": 45231, "output_size": 60308 },
    { "input": "notes.txt.b64", "output": "notes.txt", "status": "error", "error": "Invalid Base64: ..." }
  ]
}
```

**Tests:**
- Encode a directory of mixed files
- Decode a directory of `.b64` files
- Handle permission errors gracefully
- Manifest JSON round-trip (serialize → deserialize)

---

## 3. Multi-Format Detect & Convert

**What:** Auto-detect hex, Base32, Base58, URL-encoded, percent-encoded strings and offer conversion between them.

**Detectors** (extend `detect.rs`):
- Hex: `/^[0-9a-fA-F]{8,}$/` (with whitespace tolerance)
- Base32: `/^[A-Z2-7]+=*$/` (length multiple of 8)
- Base58: `/^[1-9A-HJ-NP-Za-km-z]+$/` (no 0, O, I, l)
- Percent-encoded: `/%[0-9a-fA-F]{2}/` (URL-encoded strings)

**Converter** (`convert.rs`):
- Pure functions: `hex_to_bytes`, `bytes_to_hex`, `base32_encode`, `base32_decode`, `base58_encode`, `base58_decode`, `percent_encode`, `percent_decode`
- `convert(input, source_format, target_format) -> Result<String, ConvertError>`
- Format enum: `Hex`, `Base32`, `Base58`, `PercentEncoded`, `Base64`

**UI:**
- When detection finds a non-Base64 encoding, show a hint: "Detected Hex — Convert to Base64?"
- A "Convert" dropdown button appears when detection succeeds for a non-primary format
- Target format selector in the dropdown

**New dependencies:**
- `data-encoding` crate (covers Base32, Base58, Hex in one well-tested package)

**Tests:**
- Property-based: encode → decode round-trips for each format
- Cross-format: hex → base64 → hex should be identity
- Detection accuracy on mixed-content strings

---

## 4. Diff View

**What:** Paste two Base64 strings, see their decoded content side-by-side with differences highlighted.

**Logic** (`diff.rs`):
- Takes two decoded strings (text or binary)
- Produces a list of diff hunks: `{line_number_a, line_number_b, kind: Added|Removed|Unchanged, lines}`
- Uses a simple LCS-based diff algorithm (or `similar` crate)
- For binary data: hex-dump comparison mode

**UI:**
- Toggle via keyboard shortcut (Cmd/Ctrl+D) or command palette
- Output area splits into two panels with a shared scroll
- Added lines: green background, removed lines: red background
- Line numbers on both sides
- Summary header: "3 additions, 1 removal, 42 unchanged lines"

**Activation:**
- Two Base64 strings separated by `---` or `===` delimiter in the input
- Or: paste one, click "Add comparison", paste second

**Tests:**
- Identical inputs → no differences
- Single character change → one hunk
- Different lengths → truncated comparison
- Binary diff mode (hex dump side-by-side)

---

## 5. Hash & Checksum Sidebar

**What:** Show MD5, SHA-1, SHA-256 of decoded bytes. Useful for verifying artifacts.

**Logic** (`hash.rs`):
- Compute hashes on successful decode
- Functions: `md5(bytes)`, `sha1(bytes)`, `sha256(bytes)` → hex strings
- Computed lazily (only when output is visible)

**UI:**
- Small info row below the output area (collapsible)
- Shows: `MD5: a1b2...  SHA1: c3d4...  SHA256: e5f6...`
- Each hash has a copy button (copies just that hash)
- Copy confirmation pulse animation (reuses existing pulse logic)

**New dependencies:**
- `sha2` crate (SHA-1, SHA-256)
- `md-5` crate

**Tests:**
- Known vectors: hash of "Hello, world!" matches expected values
- Empty input → no hashes shown
- Binary data → correct hashes (not UTF-8 dependent)

---

## 6. JWT Deep Inspector

**What:** Extend the existing JWT decoding with claim explanations, expiry highlighting, and structured display.

**What's already implemented:**
- JWT detection (3-part dot-separated)
- Header + payload Base64URL decoding
- Pretty-printed JSON output

**Additions:**
- Parse known claims: `exp`, `iat`, `nbf`, `iss`, `aud`, `sub`, `jti`
- Display as a structured table:
  - `exp`: "Apr 11, 2026 12:00 UTC" + badge: "Expired" (red) or "Valid" (green) or "Expiring in 2h" (yellow)
  - `iat`: "Issued: Apr 11, 2025 12:00 UTC"
  - `nbf`: "Not valid before: ..."
- Unrecognized claims shown in a raw JSON section below
- Algorithm badge from header: "HS256", "RS256", etc.

**UI:**
- When JWT is detected, output area switches to structured view instead of raw JSON
- Sections: Header (algorithm badge), Claims (table), Raw JSON (collapsible)
- No signature verification in Phase 2 (that's complex — defer to Phase 3 or optional)

**New dependencies:**
- `chrono` or `time` crate for timestamp formatting

**Tests:**
- Token with expired `exp` → shows "Expired" badge
- Token with future `nbf` → shows "Not yet valid"
- Token with all standard claims → all formatted correctly
- Malformed JWT (2 parts, 4 parts) → falls through to regular decode

---

## 7. Image Preview Upgrades

**What:** Enhance the existing image preview with metadata, dimensions, file size, and export capability.

**What's already implemented:**
- Basic image preview when decoded bytes are a valid image
- Data URI generation for encoded images

**Additions:**
- Metadata bar below preview: `1920×1080  •  245 KB  •  image/png`
- EXIF metadata extraction (via `kamadak-exif` crate)
  - Camera model, date taken, GPS coordinates (if present)
  - Shown in a collapsible "Metadata" section
- Export button → opens file picker → saves decoded image as file
- EXIF stripping option before export (privacy feature)

**UI:**
- Preview image with a subtle border/shadow
- Metadata row directly below
- "Export Image..." button next to the preview
- Optional: "Strip EXIF & Export" variant

**New dependencies:**
- `kamadak-exif` crate for EXIF parsing

**Tests:**
- PNG without EXIF → dimensions + size shown, no metadata section
- JPEG with EXIF → camera model, date extracted
- Export flow: save to temp dir, verify file exists and matches

---

## 8. Command Palette (Cmd/Ctrl+K)

**What:** Every action reachable from keyboard via a fuzzy-searchable overlay.

**Logic** (`cmd_palette.rs`):
- Registry of commands: `{name, shortcut, action: Fn(&mut Basie64App, &egui::Context)}`
- Fuzzy matcher: filters commands by typed query (subsequence match, not substring)
- Keyboard navigation: arrow keys move selection, Enter executes, Escape closes

**Registered commands (initial set):**
- Encode (Cmd+Enter)
- Decode (Cmd+Enter)
- Copy Output (Cmd+Shift+C)
- Clear All (Escape)
- Toggle Theme
- Toggle Light/Dark/System
- Open History
- Toggle Private Mode
- Batch Encode Folder
- Batch Decode Folder
- Show Diff Mode
- Copy MD5
- Copy SHA-256
- Export Image

**UI:**
- Centered overlay modal (400px wide, max 300px tall)
- Search input at top
- Scrollable list of filtered commands below
- Selected command highlighted
- Closes on: Escape, clicking outside, executing a command
- Opens with: Cmd+K / Ctrl+K

**New dependencies:**
- `fuzzy-matcher` crate (optional — can implement simple subsequence match without deps)

**Tests:**
- Fuzzy matching: "cpy" matches "Copy Output"
- Keyboard navigation: down arrow moves selection
- Command execution: triggers the right action
- Overlay closes on Escape / outside click

---

## 9. CLI Companion (`basie`)

**What:** Ship a `basie` CLI alongside the GUI using the same core crate. Power users can pipe to it. Great portfolio story: "shared core, two frontends."

**Commands:**
```
basie encode [input]        # Encode stdin or argument to Base64
basie decode [input]        # Decode stdin or argument from Base64
basie detect [input]        # Detect encoding type
basie convert <from> <to> [input]  # Convert between formats
basie hash <algorithm> [input]     # Show hash of decoded bytes
basie batch <encode|decode> <dir>  # Batch process a directory
```

**Behavior:**
- If no `[input]` given, reads from stdin
- `--variant` flag for encode/decode: `standard`, `url-safe`, `url-safe-no-pad`
- `--output` / `-o` flag for file output
- `--json` flag for machine-readable output (detect, hash, batch)
- Exit code 0 on success, 1 on error, error message to stderr

**Implementation** (`cli.rs`):
- `clap` for argument parsing with subcommands
- Dispatches to functions in `core/` module
- No `egui` dependency — pure CLI

**New dependencies:**
- `clap` (with `derive` feature for subcommand macros)

**Tests:**
- `basie encode "Hello" → "SGVsbG8="`
- `basie decode "SGVsbG8=" → "Hello"`
- `echo "Hello" | basie encode` (pipe mode)
- Error cases: invalid Base64 → stderr + exit 1
- JSON output format for `detect` and `hash`

---

## 10. Architecture Refactoring (Foundation for #9)

**What:** Split the monolith into `core/` (pure logic) and `ui/` (egui widgets). The CLI binary depends only on `core/`.

**New structure:**
```
src/
├── main.rs              # GUI binary entry point (eframe)
├── cli.rs               # CLI binary entry point (clap)
├── app.rs               # Basie64App state + eframe::App::update (thin layer)
├── theme.rs             # Theme enum + palette application
├── settings.rs          # Persisted prefs
├── samples.rs           # Hard-coded sample payloads
├── core/
│   ├── mod.rs           # Re-exports
│   ├── encode.rs        # Pure encode function
│   ├── decode.rs        # Pure decode + JWT + hint logic (moved from decode.rs)
│   ├── detect.rs        # Smart-detection scan (moved from detect.rs)
│   ├── history.rs       # History data structure + store
│   ├── batch.rs         # Batch processing logic
│   ├── convert.rs       # Multi-format conversion
│   ├── diff.rs          # Diff computation
│   └── hash.rs          # Hash computation
└── ui/
    ├── mod.rs
    ├── top_bar.rs
    ├── input.rs
    ├── buttons.rs
    ├── output.rs
    ├── banner.rs
    ├── history_panel.rs
    └── cmd_palette.rs
```

**Cargo.toml changes:**
```toml
[[bin]]
name = "basie-64"
path = "src/main.rs"

[[bin]]
name = "basie"
path = "src/cli.rs"
```

**Rules:**
- `core/` modules must have **zero** `egui` imports — pure Rust only
- `ui/` modules can import `egui` and call `core/` functions
- `app.rs` is the bridge — holds `Basie64App` state, calls `core/` and renders via `ui/`
- `decode.rs` at root level becomes a thin wrapper that delegates to `core::decode` and updates app state

**Migration order:**
1. Create `core/` directory with `mod.rs`
2. Move `detect.rs` → `core/detect.rs`, update imports
3. Move `decode.rs` pure functions → `core/decode.rs`, keep app method in `app.rs`
4. Extract encode logic → `core/encode.rs`
5. Add new modules (`history`, `batch`, `convert`, `diff`, `hash`) directly in `core/`
6. Create `cli.rs`, wire up `clap` subcommands to `core/`
7. Run `cargo test` + `cargo clippy` — must be clean

**Tests:**
- All existing tests must pass (they're already testing pure logic in `decode.rs` and `detect.rs`)
- New tests for each `core/` module independently

---

## Suggested Sub-Ordering

| # | Feature | Dependency | Effort | Why this order |
|---|---|---|---|---|
| 1 | Architecture refactoring (#10) | None | Medium | Unblocks CLI and all new `core/` modules |
| 2 | CLI companion (#9) | #10 | Small | Shared core is the portfolio story — ship it early |
| 3 | History panel (#1) | #10 | Medium | Highest user-facing value, standalone |
| 4 | Hash sidebar (#5) | #10 | Small | Quick win, independent |
| 5 | JWT inspector (#6) | None (extends existing) | Medium | Builds on current JWT code |
| 6 | Multi-format detect (#3) | #10 | Medium | Adds depth to detection story |
| 7 | Batch mode (#2) | #10 (core/batch.rs) | Large | Complex, benefits from CLI first |
| 8 | Diff view (#4) | #10 (core/diff.rs) | Medium | Niche but impressive demo |
| 9 | Image upgrades (#7) | None (extends existing) | Small | Nice-to-have polish |
| 10 | Command palette (#8) | All above | Medium | UX glue — most useful when all actions exist |

Each sub-milestone should be its own commit/PR with:
- All tests passing
- `cargo clippy --all-targets -- -D warnings` clean
- `cargo fmt` applied
- A note in `CHANGELOG.md`

---

## New Dependencies Summary

| Crate | Feature | Phase |
|---|---|---|
| `data-encoding` | Base32, Base58, Hex | #3 Multi-format |
| `sha2` | SHA-1, SHA-256 | #5 Hash |
| `md-5` | MD5 | #5 Hash |
| `clap` (derive) | CLI arg parsing | #9 CLI |
| `chrono` or `time` | Timestamp formatting | #6 JWT |
| `kamadak-exif` | EXIF metadata | #7 Image |
| `similar` (optional) | Diff algorithm | #4 Diff |
| `fuzzy-matcher` (optional) | Command palette search | #8 Palette |

All are well-maintained, zero-to-low-dependency crates that won't bloat the binary significantly.

---

## Explicit Non-Goals (reaffirmed from POLISH_PLAN.md)

- ❌ Cloud sync, accounts, telemetry by default
- ❌ Becoming a general encoding/hashing/crypto toolkit
- ❌ Mobile ports
- ❌ Monetization / paid tiers
- ❌ Web version (defeats the offline-first pitch)
- ❌ JWT signature verification (defer to Phase 3 or make optional)

---

*Created: 2026-04-11*
