# Changelog

All notable changes to Basie-64 are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.0] - 2026-04-17

The "Power User" release. Covers `POLISH_PLAN.md` Phase 1 (UX polish), Phase 2 (differentiating features), most of Phase 3 (architecture refactor and lint gating), and the full Pencil design-system migration.

### Added
- **Light, dark, and system-follow themes** with persisted preference via `directories`.
- **Empty-state hint and Samples menu** (JWT, PNG data URI, JSON-with-embedded-Base64) in the input area.
- **Friendlier, actionable error messages** and a large-paste guard.
- **History panel** with timestamped, searchable entries; reload via Enter or double-click; per-entry delete and clear-all; persisted as TOML with FIFO eviction at 200 entries (`Cmd/Ctrl+H`).
- **Private mode** toggle in the top-bar settings — suppresses history writes and tints the UI purple as a visible indicator.
- **Batch mode** — drop a folder or pick multiple files, encode or decode all of them, and review per-file status in a results table.
- **Multi-format detect & convert** — auto-detects Percent / Hex / Base32 / Base58 / Base64 with a priority scan, and round-trips between any pair of formats.
- **Diff view** — paste two payloads separated by `---` (or trigger from the palette) for a side-by-side decoded text or binary hex-dump diff.
- **JWT inspector** — structured header/claim parsing, RFC 7519 explanations, humanized `exp` / `iat` / `nbf`, warnings for `alg:none` / expired / not-yet-valid / missing `exp`, and local HMAC verification (HS256/384/512).
- **Hash sidebar** — MD5 and SHA-256 of decoded bytes, ready to copy.
- **Image upgrades** — image kind / dimensions / file-size bar, collapsible EXIF field list, and an Export Image dialog with lossless metadata stripping (EXIF segments, PNG text chunks, XMP, IPTC).
- **Command palette** (`Cmd/Ctrl+K`) — fuzzy-searchable overlay reaching every action, with keyboard navigation and shortcut hints.
- **CLI companion** — `basie` ships alongside the GUI: `encode`, `decode`, `convert`, `detect`, `diff`, `hash` subcommands with stdin and `--output` support.
- **Pencil design system** — `Tokens` palette, shared widget helpers (`primary_button`, `card_frame`, `accent_banner`, `glass_panel`, …), embedded Inter / IBM Plex Mono / Lucide fonts, Arc-style glassmorphism pass across every screen.
- **CI workflow** running `cargo test`, `cargo clippy -D warnings`, and `cargo fmt --check` on every push and PR to `master`.

### Changed
- **Architecture refactor** — pure logic moved into `src/core/` (zero `egui` imports); `src/ui/` modules read and mutate `Basie64App` directly; the CLI links only `core/`.
- **Theme bootstrap** centralized in `theme::apply` + `theme::install_fonts`; ad-hoc style mutations removed from UI code.
- **Output panel** redesigned with the Pencil tokens — frame, image meta bar, and JWT inspector card all share the new design language.
- **Spacing, padding, and font pairing pass** for a consistent, native feel.
- **Bundle metadata** in `Cargo.toml` synced with the package version (was drifting between `0.1.0` and `0.2.0`).

### Fixed
- Decode confirmation and repaint regressions inherited from earlier WIP branches.

## [0.2.0] - 2026-03-10

### Added
- Multi-platform release pipeline via GitHub Actions producing macOS, Linux, and Windows binaries on `v*` tags.
- Frameless, transparent window with drag-handle and close button.
- JWT and data URI decoding, multi-variant Base64 fallback (Standard / URL-safe / URL-safe-no-pad).
- Keyboard shortcuts and scrollable text areas.

## [0.1.0] - 2026-03-10

Initial release. Core Base64 encoding and decoding with file drop and image preview.

[Unreleased]: https://github.com/Mclovin0213/basie-64/compare/v0.4.0...HEAD
[0.4.0]: https://github.com/Mclovin0213/basie-64/compare/v0.2.0...v0.4.0
[0.2.0]: https://github.com/Mclovin0213/basie-64/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/Mclovin0213/basie-64/releases/tag/v0.1.0
