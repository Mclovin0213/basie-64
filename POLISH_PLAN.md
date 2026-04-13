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

- **History panel** ✅ — timestamped, searchable list of recent encodes/decodes (local only, clearable, with a "private mode" toggle for sensitive data). Implemented in `v0.3.1`:
  - `src/core/history.rs` — `HistoryEntry` + `HistoryStore` (JSON persistence, FIFO eviction at 200 entries, search filtering, stable entry IDs, full-input reload support)
  - `src/ui/history_panel.rs` — Collapsible bottom panel with dedicated search state, row selection, Enter/double-click reload, per-entry delete, clear all
  - Keyboard shortcut: Cmd/Ctrl+H to toggle
  - Private mode toggle in top-bar settings (persisted in `settings.private_mode`)
  - First `core/` module — pure logic, zero `egui` imports (unblocks CLI companion)
  - Tests cover append, eviction, search, private mode, exact delete, full-input reload, and encode/decode round-trip restore
- **Batch mode** — drop a folder, encode/decode every file, export as a manifest
- **Multi-format detect & convert** — auto-detect hex, Base32, Base58, URL-encoded, percent-encoded, and offer conversion
- **Diff view** — paste two Base64 strings, see decoded diff side-by-side
- **Hash + checksum sidebar** — show MD5/SHA-1/SHA-256 of decoded bytes (useful for verifying artifacts)
- **JWT deep inspector** ✅ — structured parse, RFC 7519 claim explanations, humanized `exp`/`iat`/`nbf`, warnings (`alg:none`, expired, not-yet-valid, issued-in-future, missing `exp`), and local HMAC signature verification (HS256/384/512). Implemented in `src/core/jwt.rs` (pure, zero egui — CLI and GUI share it). Inspector card renders below the output text area in `src/ui/output.rs`. Asymmetric verification (RS256/ES256) is a follow-up.
- **Image preview upgrades** — EXIF stripping, dimensions, file size; optional export
- **Command palette** (Cmd/Ctrl+K) — every action reachable from keyboard
- **CLI companion** — ship a `basie` CLI alongside the GUI using the same core crate, so power users can pipe to it. Great portfolio story: "shared core, two frontends."
- **Plugin/extension-ready architecture** *(optional, only if it falls out naturally)* — don't over-engineer, but splitting core logic from UI makes the CLI above trivial.

---

## Phase 3 — Code Quality & Reliability

Goal: the codebase reviews well in a portfolio context.

- **Refactor the monolith** — split `main.rs` into `core/` (encoding, detection, JWT), `ui/` (egui widgets), and `app.rs` (state + wiring). Keep it pragmatic, not over-architected.
- **Expand test coverage**
  - Property-based tests (via `proptest`) for round-trip encode/decode
  - Snapshot tests for UI state transitions
  - Fuzz target for the detection/scanner logic
- **Linting & formatting**
  - `cargo fmt` enforcement in CI
  - `cargo clippy -- -D warnings` gate
  - `cargo deny` for license + security audit
- **Error handling audit** — replace any `unwrap`/`expect` in user-facing paths with real errors
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

| Milestone | Phases | Output |
|---|---|---|
| **v0.3 — "Feels Finished"** | 1, partial 3 | Polished UX, light mode, settings, clean codebase |
| **v0.4 — "Power User"** | 2, rest of 3 | History, batch mode, CLI companion, full test suite |
| **v1.0 — "Shippable"** | 4, 5 | Branding, installers, signing, auto-update |
| **v1.0 Launch** | 6, 7 | Docs, landing page, case study, public launch |

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

*Last updated: 2026-04-11*
