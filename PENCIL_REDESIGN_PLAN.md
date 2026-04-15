# Basie-64 — Pencil Design Implementation Plan

> **Status (2026-04-13):** Phase 1 (Foundation) has landed on `feature/design-tokens`.
> The token layer (`src/theme.rs` → `Tokens` struct), font loader
> (`theme::install_fonts`), Lucide icon constants (`theme::icons`), and shared
> widget helpers (`src/ui/widgets.rs`) all exist and compile clean; the existing
> UI still renders with the old layout. Phases 2-7 (per-screen migrations,
> listed in the table below) are pending — pick up from there.

## Context

The Pencil file `/Users/bugz_mac/Desktop/positive_art/-/pencil/basie_64.pen` ships a polished visual redesign of basie-64 across five screens (Empty State, JWT Decoded, History Panel Open, Image Decoded + Metadata, Export Image Dialog). The current eframe app uses ad-hoc emoji icons, a couple of hardcoded `Color32` values per theme, and stock egui widget styling. The new design is a coherent system: a 24-token color palette, four radii, six spacing steps, **Inter** + **IBM Plex Mono** typography, **Lucide** icon glyphs, glass overlays, sectioned cards with dividers, and a refined chrome-style top bar. The goal is to migrate every screen to that design without changing app behavior or breaking the `core/` ↔ `cli` boundary.

## Goals

1. Stand up a real design-token layer in Rust that mirrors the Pencil variables so future tweaks are one-line edits.
2. Replace the emoji-driven UI with embedded Inter + IBM Plex Mono + Lucide so the app looks the same across macOS / Windows / Linux without external font deps.
3. Refactor each `src/ui/*.rs` file to consume shared widget helpers (`primary_button`, `card_frame`, `accent_banner`, `meta_chip`, …) instead of bespoke `Frame`/`Button` calls, so the design is enforceable.
4. Land it without touching `src/core/**` (the CLI must keep linking) and without the privacy / offline guarantees regressing.

## Design tokens to lift verbatim from `basie_64.pen`

```
Colors (dark)
  --bg-base #0D0F12   --bg-surface #14161B   --bg-elevated #1A1D24
  --bg-card #1E2128   --bg-input #12141A     --bg-hover #252830
  --border-subtle #2A2D36   --border-default #363944   --border-focus #5B9BD5
  --text-primary #E8EAED    --text-secondary #9BA1AD   --text-muted #6B7280
  --text-mono #C4CAD4
  --accent-blue #5B9BD5  / -dim #2A3F55
  --accent-amber #D4B06A / -dim #3A3220
  --accent-green #7ABFA0 / -dim #1E3A2F
  --accent-orange #D4A574 / -dim #3A2E1E
  --accent-purple #A78BDB / -dim #2A1E3A
  --accent-red #D48A8A   / -dim #3A1E1E
  --btn-primary-bg #5B9BD5  --btn-primary-text #0D0F12
  --btn-secondary-bg #1E2128 --btn-secondary-text #C4CAD4
  --btn-ghost-text #9BA1AD
  --history-bg #111318CC   --modal-backdrop #0D0F1299
  --modal-surface #1A1D24  --panel-glass #14161BDD

Radii   sm 4 · md 6 · lg 8 · xl 12
Spacing xs 4 · sm 8 · md 12 · lg 16 · xl 24 · 2xl 32
Type    Inter (UI), IBM Plex Mono (code/hint/size readouts)
Icons   Lucide (sun, settings, clock-3, x, scan-eye, layers, trash-2,
        download, triangle-alert, chevron-right, binary, …)
```

Everything below references these names.

## Foundation work (`src/theme.rs` rewrite)

Today `theme.rs` is ~110 LOC with two hardcoded `Visuals` blocks. Rewrite it as the single source of truth for the token system:

1. **`Tokens` struct** — one `pub const` for every variable above, with a `Tokens::dark()` and `Tokens::light()` factory. Light mode is *not* in the Pencil file yet, so derive a sensible inversion (white surfaces, darker text) and gate it behind a TODO comment so we don't pretend it's pixel-perfect.
2. **`apply(ctx, theme)`** — keep the function signature, but build `egui::Visuals` from `Tokens`: `panel_fill = bg_base`, `widgets.noninteractive.bg_fill = bg_card`, `widgets.inactive.bg_fill = bg_input`, `widgets.hovered.bg_fill = bg_hover`, `selection.bg_fill = accent_blue`, `selection.stroke = border_focus`, `window_fill = modal_surface`, `window_stroke = border_default`, etc. Set `style.spacing.item_spacing = vec2(8, 8)`, `button_padding = vec2(14, 8)`, `window_rounding = 12`, `menu_rounding = 8`.
3. **Font loader** — new `pub fn install_fonts(ctx)` called once from `main.rs` after `apply`. Embeds three TTFs via `include_bytes!`:
   - `assets/fonts/Inter-Regular.ttf` → family `"inter"`, default proportional
   - `assets/fonts/Inter-SemiBold.ttf` → family `"inter_semibold"` (used for headings / button labels)
   - `assets/fonts/IBMPlexMono-Regular.ttf` → family `"plex_mono"`, default monospace
   - `assets/fonts/lucide.ttf` → family `"lucide"`, used by `icons` module
4. **`pub mod icons`** — a thin module that exposes constants (`pub const SUN: char = '\u{e9d2}';` …) for the ~20 Lucide glyphs the design uses, so call sites do `ui.label(RichText::new(icons::HISTORY).family("lucide".into()))`. Pull the codepoints from `lucide-static`'s `info.json` once and check them in alongside the font.
5. **`top_bar_fill` / accent helpers** — keep the existing `top_bar_fill(theme)` API but back it by `Tokens::current().bg_surface` so callers don't break.

## Shared widget helpers (new `src/ui/widgets.rs`)

Today every UI file rolls its own buttons / cards / banners. Add a small module of pure helper functions that all consume `&mut egui::Ui` and return `egui::Response`. Keep them dumb — no app state, no mutation of `Basie64App`.

```rust
pub fn primary_button(ui: &mut Ui, label: &str, icon: Option<char>) -> Response;
pub fn secondary_button(ui: &mut Ui, label: &str, icon: Option<char>) -> Response;
pub fn ghost_button(ui: &mut Ui, label: &str, icon: Option<char>) -> Response;
pub fn icon_button(ui: &mut Ui, icon: char, tooltip: &str, active: bool) -> Response;
pub fn card_frame(ui: &mut Ui, add: impl FnOnce(&mut Ui)) -> Response;            // bg-card + border-subtle + radius-lg
pub fn input_frame(ui: &mut Ui, add: impl FnOnce(&mut Ui)) -> Response;           // bg-input + border-subtle + radius-md + padding 16
pub fn glass_panel(ui: &mut Ui, add: impl FnOnce(&mut Ui));                       // history-bg + radius [12,12,0,0] + border-subtle
pub fn divider(ui: &mut Ui);                                                       // 1px border-subtle row
pub fn accent_banner(ui, accent: AccentTone, icon, text, action: Option<(&str, &mut bool)>);
pub fn key_chip(ui: &mut Ui, key: &str, label: &str);                              // bottom hint-row pill
pub fn meta_chip(ui: &mut Ui, kind: &str);                                         // small filled rectangle for "PNG" etc.
pub fn section_header(ui: &mut Ui, label: &str);                                   // 13px Inter SemiBold $text-primary
```

These are the "shared components" the user asked about — once they exist, every screen migration becomes a localized rewrite.

## Per-screen migration (drives the work in `src/ui/`)

For each screen below, the "do" column lists concrete edits in existing files. Numbers in parentheses are Pencil node IDs so anyone executing the plan can re-screenshot the source of truth.

| Screen | File(s) to edit | Do |
|---|---|---|
| **Top bar** (`Bahcc`/`UBM2M`/`HQrj9`) | `src/ui/top_bar.rs` | Replace the current draggable label with a 48px row, padding `[0,16]`, fill `bg_surface`, 1px bottom border `border_subtle`. Left lockup: 24×24 `accent_blue` rounded-md tile with `lucide::BINARY` glyph + "Basie-64" Inter SemiBold 15. Right cluster: 4× `icon_button(32×32, radius_md)` for theme / settings / history / close, gap 4. The history button uses `active=true` styling (`accent_blue_dim` fill) when `app.show_history_panel` is on — that state was missing before and the design calls for it. Drop the legacy theme cycle button label; the icon swaps between `SUN`/`MOON`/`MONITOR` based on resolved theme. |
| **Empty state** (`Bahcc`) | `src/ui/input.rs`, `src/ui/buttons.rs`, `src/ui/output.rs`, `src/app.rs` | Wrap input + output in `input_frame()` (radius-md, padding 16, bg-input, border-subtle). Replace the existing samples popup with a `ghost_button` in the top-right of the input frame using `lucide::PACKAGE`. The hint row at the bottom of the screen becomes a footer panel rendered in `app.rs` (`TopBottomPanel::bottom`) using `key_chip()` × N — that's a behavior delta worth flagging: hints currently live inside the empty-state placeholder; the design treats them as a persistent footer. |
| **Action row** (all screens) | `src/ui/buttons.rs` | Rebuild as: `secondary_button("Encode", ENCODE_ICON)`, `primary_button("Decode", DECODE_ICON)`, `ghost_button("Diff", DIFF_ICON)`, vertical 1px×20 divider (use `Frame::none().fill(border_subtle)`), `ghost_button("Save as File", DOWNLOAD)`, `ghost_button("Clear", TRASH_2)`, then `ui.add_space(ui.available_width() - …)` pushes the two `secondary_button` batch entries flush right. The "primary" affordance flips between Encode and Decode based on `app.last_action_was_encode` so the design's "the primary action highlights the next likely click" pattern works. |
| **JWT Decoded** (`UBM2M`) | `src/ui/output.rs` (`jwt_inspector` submodule), `src/ui/banner.rs` | The new banner uses `accent_banner(AccentTone::Blue, SCAN_EYE, "JWT Token Detected", Some(("Decode It →", &mut decode_clicked)))`. Rebuild the inspector as a `card_frame` containing four sections separated by `divider()`s: Header Claims (table of bg-elevated rows, 8×12 padding, border-subtle bottom), Payload Claims (same), warnings strip (`accent_orange_dim` fill, `accent_orange` text, `TRIANGLE_ALERT` icon), HMAC verification (CollapsingHeader styled with `lucide::CHEVRON_RIGHT`, secret `TextEdit` inside an `input_frame`, ghost "Verify Signature" button). Re-use `core::jwt` as-is — only the rendering changes. |
| **History panel** (`srORL`) | `src/ui/history_panel.rs`, `src/app.rs` | Major change: it is no longer a `TopBottomPanel::bottom`, it is a glass overlay anchored to the bottom of the central area. Use `egui::Area::new("history_panel").anchor(Align2::CENTER_BOTTOM, ..)` containing a `glass_panel(...)` with: 4×40 grab-handle rectangle on top, header row "History · 12 entries · Clear All ✕", search field rendered via `input_frame`, scrollable list where each entry is a 6-radius rounded row (selected entry: `accent_blue_dim` fill + `accent_blue` 1px border, 10×12 padding, gap 12, label/preview text). Hooks into existing `core::history::HistoryStore` unchanged. |
| **Image Decoded + Metadata** (`HQrj9`) | `src/ui/output.rs` (`image_meta_bar` submodule) | Restructure into three stacked frames. (1) `imagePreview`: a `Frame::canvas` with top-only radius `[8,8,0,0]`, `border_subtle`, fixed height matching `image_preview` aspect — currently it just draws an `Image` widget; we need to wrap it in a frame that matches the design. (2) `metadataBar`: 40px tall `bg_card` row with `meta_chip("PNG")` + dimensions text + size text + spacer + small `EXIF count` ghost button + "Export…" `secondary_button`. (3) `exifCard`: existing CollapsingHeader content moved into a `card_frame`. Bottom of central panel gets `"Decoded · {mime} · {size}"` line in `text_muted` Plex Mono 11. |
| **Export Image dialog** (`8FNcc`) | `src/ui/export_image_dialog.rs` | Reskin only. Backdrop `modal_backdrop`, modal `Frame::window` with `modal_surface` fill, 1px `border_default`, radius 12, two-layer drop shadow (`y:8 blur:40 #00000055` + `y:2 blur:8 #00000033`). Body sections: header with title + close icon button, divider, image readout `input_frame`, metadata section (re-uses the EXIF list from `image_meta_bar`), strip checkbox row, divider, footer (esc hint left via `key_chip`, Cancel `secondary_button` + Save `primary_button` right). |
| **Bottom hint row / status** | `src/app.rs`, `src/ui/widgets.rs` | New persistent `TopBottomPanel::bottom` ~32px tall, fill `bg_surface`, 1px top border. Renders `key_chip("⌘↵","encode/decode")`, `key_chip("⌘D","diff")`, `key_chip("⌘K","commands")`, `key_chip("⌘H","history")` — the `H` chip uses the accent-blue active style when the panel is open. Replaces the in-input "Got it" dismissable hint, which can be deleted along with the `shortcut_hint_dismissed` setting. |

## Asset & dependency changes

- **New directory** `assets/fonts/` containing:
  - `Inter-Regular.ttf` (OFL, ~310 KB)
  - `Inter-SemiBold.ttf` (OFL)
  - `IBMPlexMono-Regular.ttf` (OFL)
  - `lucide.ttf` (ISC, ~80 KB) + `lucide-codepoints.json` (subset of `lucide-static/font/info.json`, just the ~20 names we use)
- **Cargo.toml**: no new runtime deps required — fonts go through `egui::FontDefinitions` via `include_bytes!`. Add a `build.rs`-free comment in the new `src/theme/icons.rs` documenting how to regenerate the codepoint list when bumping Lucide.
- **`.gitignore`**: leave alone; fonts are checked in.
- **License notes**: add `assets/fonts/LICENSE-INTER`, `LICENSE-IBM-PLEX`, `LICENSE-LUCIDE` text files so the offline-first / privacy-friendly story stays credible.
- **No new crates.** Resist the urge to add `egui_phosphor` or `lucide-rs`; both pull in build-time tooling we don't need for a 20-glyph subset.

## Files this plan touches

```
src/theme.rs                          rewrite (tokens + fonts + icons module)
src/main.rs                           call install_fonts after apply
src/app.rs                            add bottom status panel, history Area anchor
src/ui/mod.rs                         export new widgets module
src/ui/widgets.rs                     NEW — shared helpers
src/ui/top_bar.rs                     replace
src/ui/input.rs                       wrap in input_frame, drop empty-state hint
src/ui/buttons.rs                     replace with shared button helpers + flip primary by context
src/ui/output.rs                      restructure image_meta_bar + jwt_inspector
src/ui/banner.rs                      use accent_banner helper
src/ui/history_panel.rs               glass overlay rewrite
src/ui/export_image_dialog.rs         reskin to match modal spec
assets/fonts/                         NEW (4 TTFs + license files + codepoints json)
src/settings.rs                       drop shortcut_hint_dismissed (no longer needed)
```

`src/core/**` is untouched. `src/cli.rs` is untouched.

## Phasing (for execution after this plan is approved)

1. **Foundation** — ✅ *shipped on `feature/design-tokens`*. `theme.rs` rewritten around `Tokens` (dark verbatim from the Pencil file, light derived and marked provisional), fonts dropped in `assets/fonts/`, `widgets.rs` added with the helpers listed above, `install_fonts` wired from `main.rs`. Verified: `cargo clippy --all-targets -- -D warnings` clean, `cargo test` 25/25, `basie encode "Hello" → SGVsbG8=`, GUI launches with the old layout drawn through the new token pipeline.
2. **Top bar + status footer** — ✅ *shipped on `feature/design-tokens` migrate `top_bar.rs`, add the bottom hint panel via `app.rs`. Verify visually against `Bahcc` screenshot.
3. **Input / buttons / output** — ✅ *shipped on `feature/pencil-phase2-topbar-footer`*. `input.rs` wrapped in `input_frame`, `buttons.rs` rebuilt with token helpers + primary/secondary flip, `output.rs` top-level wrapped in `input_frame` with token copy buttons.
4. **Banners + JWT inspector** — ✅ *shipped on `feature/pencil-phase2-topbar-footer`*. `banner.rs` uses `accent_banner` for all 4 banner types; `jwt_inspector` rebuilt as `card_frame` with `section_header`s, `divider`s, and `accent_banner` warnings.
5. **Image meta bar** — ✅ *shipped on `feature/pencil-phase2-topbar-footer`*. Restructured into preview frame + meta row + EXIF `card_frame`.
6. **Export modal** — ✅ *shipped on `feature/pencil-phase2-topbar-footer`*. Reskinned with `modal_surface`, `border_default`, radius 12, shadow, header with `icon_button(X)`, `key_chip` footer.
7. **History overlay** — ✅ *shipped on `feature/pencil-phase2-topbar-footer`*. `TopBottomPanel::bottom` replaced with `egui::Area` glass overlay anchored above the status footer.
8. **Sweep** — ✅ *shipped on `feature/pencil-phase2-topbar-footer`*. `cargo fmt` + `cargo clippy --all-targets -- -D warnings` clean, `cargo test` 152/152, `basie encode "test" → dGVzdA==`.

Each phase ends with a screenshot of the running app placed next to the corresponding Pencil frame for visual diff.

## Verification

- `cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test` must be clean.
- `cargo run --bin basie -- encode "Hello"` must still print `SGVsbG8=` (proves `core/` ↔ `cli` boundary intact).
- `cargo run` and walk the keyboard tour: type input → ⌘↵ decode → toggle history with ⌘H → toggle command palette ⌘K → drag in a PNG data URI → click Export… → strip metadata → save. Compare each state visually to the corresponding Pencil frame using `mcp__pencil__get_screenshot`.
- Toggle theme cycle — confirm the system theme path still works (token light mode is intentionally rough; flag that as follow-up).
- Open a JWT sample, click "Decode It →", verify the inspector matches `UBM2M` section-by-section.

## Decisions locked in

- **Light theme**: ship a derived inversion now, mark it `// TODO(design): provisional — extend .pen file` in `Tokens::light()`.
- **Bottom hint footer**: persistent 32px `TopBottomPanel::bottom`. Drop `settings.shortcut_hint_dismissed` and the in-input "Got it" flow.
- **Fonts**: embed Inter Regular, Inter SemiBold, IBM Plex Mono Regular, and Lucide TTFs under `assets/fonts/` with their license files. Single self-contained binary.

## Known risks

1. **Lucide font codepoints** — Lucide ships SVG by default; the TTF lives in `lucide-static/font/`. We commit a hand-curated 20-glyph subset's codepoints. Mitigation: pin the Lucide release in a comment near `theme/icons.rs` so the codepoints can be regenerated cleanly on upgrade.
2. **History panel resizing** — the Pencil mock shows a grab handle implying drag-resize. `egui::Area` doesn't support that natively. V1 keeps it fixed at 340px and the handle is decorative; flag a follow-up to wire a manual drag handler if the user misses it.
