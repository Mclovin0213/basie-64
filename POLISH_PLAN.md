# Basie-64 Polish & Distribution Plan

A multi-phased roadmap to take Basie-64 from a functional v0.2.0 Rust/egui utility into a portfolio-worthy, distributable product.

---

## Guiding Principles

- **Best-in-class for a narrow job.** Basie-64 is a Base64 tool. It shouldn't grow into a general "encoder swiss army knife" — instead, it should be *the* Base64 tool a developer reaches for.
- **Offline-first, privacy-respecting.** Unlike the web-based converters it competes with, a native app can credibly promise "your data never leaves your machine." Lean into that.
- **Portfolio-grade polish.** Every surface a recruiter or reviewer sees (README, screenshots, release page, first-launch experience) should feel intentional.

---

## Phase 1 — Core UX Polish

Goal: the app feels finished the moment you open it.

- **First-run experience**
  - Empty-state illustrations / hint text in the input area ("Paste Base64, drop a file, or try an example")
  - Onboarding tooltip pass on the keyboard shortcuts
  - Sample payloads button (JWT example, image example, JSON-with-embedded-Base64 example)
- **Visual refinement**
  - Consistent spacing, padding, and alignment pass
  - Light mode + dark mode (currently dark-only) with a system-follow toggle
  - Font pairing review (monospace for data, proportional for chrome)
  - Subtle motion: fade-in on detection banners, copy-confirmation pulse
  - Respect reduced-motion / high-contrast OS settings
- **Error & edge-case messaging**
  - Friendly, actionable errors (e.g. "This looks like URL-safe Base64 — want to try that variant?")
  - Graceful handling of enormous pastes (progress indicator, size warnings)
- **Accessibility sweep**
  - Keyboard-only navigation audit
  - Screen-reader labels on interactive elements
  - Color contrast check (WCAG AA minimum)
- **Settings / Preferences**
  - Persisted theme, default encoding variant, recent-files list
  - Config file in standard OS location (`~/.config/basie-64/`, `%APPDATA%`, etc.)

---

## Phase 2 — Differentiating Features

Goal: things competing web tools *can't* easily do, that make this the obvious pick.

- **History panel** ✅ — timestamped, searchable list of recent encodes/decodes (local only, clearable, with a "private mode" toggle for sensitive data).
  - `src/core/history.rs` — `HistoryEntry` + `HistoryStore` (TOML persistence, FIFO eviction at 200 entries, search filtering, stable entry IDs, full-input reload support)
  - `src/ui/history_panel.rs` — Collapsible bottom panel with dedicated search state, row selection, Enter/double-click reload, per-entry delete, clear all
  - Keyboard shortcut: Cmd/Ctrl+H to toggle
  - Private mode toggle in top-bar settings (persisted in `settings.private_mode`). UX is minimal — see "Known gaps" below.
- **Batch mode** ✅ — drop a folder (or select multiple files), encode or decode every file, view a results table with per-file status.
  - `src/core/batch.rs` — `BatchOp` / `BatchSource` / `BatchConfig` / `BatchPreview` / `BatchProgress` / `BatchResult`, threaded `process_batch_with_progress`.
  - `src/ui/batch_panel.rs` — preview confirmation, progress indicator, results table.
- **Multi-format detect & convert** ✅ — auto-detect Percent / Hex / Base32 / Base58 / Base64 and offer conversion between them.
  - `src/core/detect.rs` — priority scan (Percent → Hex → Base32 → Base58 → Base64) returning a `DetectionResult` with banner text and mixed-content matches.
  - `src/core/convert.rs` — round-trip between every pair of the supported formats.
  - `src/ui/banner.rs` — detection banner with "Convert to Base64?" suggestion when a non-Base64 format is matched.
- **Diff view** ✅ — paste two Base64 strings, see decoded diff side-by-side (text or binary hex-dump).
  - `src/core/diff.rs` — `parse_diff_input` splits on `\n---\n` / `\n===\n`; `diff_text` uses the `similar` crate; `diff_binary` produces a byte-aligned hex-dump comparison.
  - `src/ui/diff_view.rs` — full-screen side-by-side view with additions/removals/unchanged summary.
  - Activation: delimiter in the input, or Cmd+D from the command palette.
- **Hash + checksum sidebar** ✅ — show MD5 / SHA-256 of decoded bytes.
  - `src/core/hash.rs` — `md5_hex`, `sha256_hex`, `sha256_base64`.
  - ⚠️ SHA-1 from the original spec is *not* implemented. If SHA-1 still matters for artifact verification, add it here.
- **JWT deep inspector** ✅ — structured parse, RFC 7519 claim explanations, humanized `exp`/`iat`/`nbf`, warnings (`alg:none`, expired, not-yet-valid, issued-in-future, missing `exp`), and local HMAC signature verification (HS256/384/512). Implemented in `src/core/jwt.rs` (pure, zero egui — CLI and GUI share it). Inspector card renders below the output text area in `src/ui/output.rs`. Asymmetric verification (RS256/ES256) is a follow-up.
- **Image preview upgrades** ✅ — shipped in a60946a. `src/core/image_meta.rs` parses image kind (PNG/JPEG/GIF/WebP/BMP/ICO), dimensions, and EXIF fields via `kamadak-exif`, and performs lossless metadata stripping (EXIF segments, PNG text chunks, XMP, IPTC). The metadata bar renders below the preview in `src/ui/output.rs` (`image_meta_bar` submodule) with kind · WxH · size, a collapsible EXIF field list, and an Export… button. The `src/ui/export_image_dialog.rs` modal handles saving with an optional "Strip metadata before saving" checkbox; the Cmd+K "Export Image" command routes through the same dialog. Asymmetric image formats (SVG, AVIF) are still a follow-up.
- **Command palette** (Cmd/Ctrl+K) ✅ — every action reachable from keyboard.
  - `src/core/command_registry.rs` — 15 registered commands with id / name / keywords / shortcut, plus `filter_commands` fuzzy matcher.
  - `src/ui/command_palette.rs` — centered overlay, search input, arrow/enter/escape navigation, dispatches to `Basie64App` methods.
- **CLI companion** ✅ — `basie` CLI ships alongside the GUI via a second `[[bin]]` in `Cargo.toml`, linking only `core/`.
  - `src/cli.rs` — clap subcommands: `encode`, `decode`, `convert`, `detect`, `diff`, `hash`. Supports stdin input and `--output` file writes.
- **Shared-core architecture** ✅ — the `core/` vs `ui/` split that this roadmap anticipated is done. The CLI reuses every piece of logic the GUI uses; no duplication.

### Phase 2 — Known gaps

- **Private mode UX is minimal.** The toggle lives only in the top-bar settings menu. There's no visible banner, no session-only override, and no clear indicator that history writes are currently suppressed. If this feature is user-facing, it needs surfacing.
- **JWT asymmetric verification (RS256/ES256)** remains a follow-up (as originally noted).

---

## Phase 3 — Code Quality & Reliability

Goal: the codebase reviews well in a portfolio context.

- **Refactor the monolith** ✅ — `main.rs` / `app.rs` / `core/` / `ui/` split is done. The `core/` boundary is now load-bearing for the CLI.
- **Expand test coverage** — unit tests exist across `core/` but the following are still pending:
  - Property-based tests (via `proptest`) for round-trip encode/decode and cross-format conversion
  - Snapshot tests for UI state transitions
  - Fuzz target for the detection/scanner logic
- **Linting & formatting**
  - `cargo fmt` enforcement in CI — status unverified, check the workflow
  - `cargo clippy -- -D warnings` gate — runs locally clean; CI enforcement unverified
  - `cargo deny` for license + security audit — not configured
- **Error handling audit** — spot-audit pending. No remaining `unwrap`/`expect` calls were flagged during the docs-cleanup pass, but a focused sweep across every user-input path hasn't been done.
- **Performance pass** — benchmark large-file encode/decode, streaming where it matters
- **Crash reporting** — optional, opt-in only (respect the privacy pitch)

---

## Phase 4 — Branding & Identity

Goal: the app has a memorable "face" — critical for portfolio impact.

- **Name + tagline lock-in** — e.g. *"Basie-64 — the Base64 tool that stays on your machine."*
- **Logo / icon redesign** — commission or craft a distinctive mark; export full icon set (16, 32, 64, 128, 256, 512, 1024 px, plus `.ico`, `.icns`, `.png`)
- **Color palette + type system** documented in a short `BRAND.md`
- **Screenshots & GIFs** — clean, consistent backgrounds, annotated for the README and release notes
- **Short demo video / animated GIF** — 15-30 seconds showing the smart-detection flow
- **Landing page** (GitHub Pages or a one-pager on your personal site)
  - Hero, feature grid, download buttons per-OS, screenshots, FAQ
  - SEO basics (og tags, favicon, sitemap)

---

## Phase 5 — Packaging & Distribution

Goal: one-click install on every major platform.

- **macOS**
  - `.app` bundle + `.dmg` with a styled background
  - Apple Developer ID signing + notarization
  - Homebrew cask (`brew install --cask basie-64`)
- **Windows**
  - MSI or NSIS installer (via `cargo-wix` or `tauri-bundler`-style tooling)
  - Authenticode code signing
  - Winget + Chocolatey submissions
- **Linux**
  - AppImage (universal)
  - `.deb` and `.rpm` packages
  - Flatpak (Flathub submission — highest reach)
  - Optional: AUR package
- **Auto-updates** — self-updater via GitHub Releases feed (e.g. `self_update` crate); opt-in, transparent
- **CI hardening**
  - Extend existing GitHub Actions to produce all the above artifacts on tag
  - Checksum + SBOM generation
  - Reproducible builds where feasible
- **Versioning & changelog**
  - Adopt SemVer strictly
  - `CHANGELOG.md` following Keep-a-Changelog format
  - Conventional commits (optional) to automate changelog

---

## Phase 6 — Documentation & Community

Goal: a newcomer can understand, install, use, and contribute in under 5 minutes.

- **README overhaul**
  - Hero image / logo
  - Animated demo GIF
  - Badges: build, release, license, platforms
  - Install instructions per OS (one-liner each)
  - Feature list with screenshots
  - Keyboard shortcut cheat-sheet
  - Privacy statement (front and center)
- **Additional docs**
  - `CONTRIBUTING.md` — dev setup, code style, PR flow
  - `CODE_OF_CONDUCT.md`
  - `SECURITY.md` — how to report issues
  - `ARCHITECTURE.md` — short tour of the crate structure (great for portfolio reviewers)
- **GitHub repo hygiene**
  - Issue + PR templates
  - Good-first-issue labels
  - Pinned discussion or roadmap
  - Release notes that read like product updates, not dumps

---

## Phase 7 — Portfolio & Launch

Goal: extract maximum career value from the work.

- **Case study write-up** on your personal site
  - Problem framing ("web Base64 tools leak sensitive data")
  - Design decisions (why Rust, why egui, why offline-first)
  - Technical highlights (smart detection algorithm, shared-core CLI+GUI, cross-platform packaging pipeline)
  - Screenshots, metrics (binary size, startup time, test coverage)
  - What you'd do differently
- **Launch checklist**
  - Post to Hacker News (Show HN), Reddit r/rust, r/programming, Lobsters
  - Submit to awesome-rust, awesome-egui lists
  - Short Twitter/Mastodon/Bluesky thread with the demo GIF
  - Dev.to or personal blog post matching the case study
- **Resume one-liner** — draft 2-3 variants emphasizing scope, tech, and reach (e.g. "Designed, built, and shipped a cross-platform Rust desktop utility with a custom smart-detection engine, distributed via Homebrew, Winget, and Flathub.")
- **Metrics to track** (optional, for the resume bullet)
  - GitHub stars, downloads per release, install counts from package managers

---

## Suggested Ordering & Milestones

| Milestone | Phases | Status | Output |
|---|---|---|---|
| **v0.3 — "Feels Finished"** | 1, partial 3 | ✅ shipped | Polished UX, light mode, settings, clean codebase |
| **v0.4 — "Power User"** | 2, rest of 3 | 🟡 nearly shipped | History, batch mode, diff view, multi-format detect/convert, JWT inspector, hash, command palette, CLI companion, image metadata + Export Image dialog. **Remaining:** property/fuzz/snapshot tests, private-mode UX. |
| **v1.0 — "Shippable"** | 4, 5 | ⏳ next focus | Branding, installers, signing, auto-update |
| **v1.0 Launch** | 6, 7 | ⏳ | Docs, landing page, case study, public launch |

Treat the milestones as checkpoints, not deadlines — ship each when it actually feels done.

---

## Explicit Non-Goals

To keep scope honest:

- ❌ Cloud sync / accounts / telemetry by default
- ❌ Becoming a general encoding/hashing/crypto toolkit
- ❌ Mobile ports (interesting, but massive scope creep)
- ❌ Monetization / paid tiers
- ❌ Web version (defeats the offline-first pitch)

---

## Version drift — needs reconciling before the next release

Three different version numbers are currently in play:

- `Cargo.toml` `[package]` → `version = "0.2.0"`
- `Cargo.toml` `[package.metadata.bundle]` → `version = "0.1.0"`
- This roadmap's milestone table → v0.3 shipped, v0.4 nearly shipped

Given the feature set actually in the tree, the `[package]` version is the one most out of date. Pick a real cadence (e.g. bump `[package]` to `0.4.0` and sync the bundle metadata) during the next release cut so the `CHANGELOG.md` (Phase 5) has something honest to reference.

---

*Last updated: 2026-04-12*
