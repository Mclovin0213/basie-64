# Arc-Style Glassmorphism Redesign

## Context

Basie-64 currently uses a flat design with 1px borders on every surface and zero shadows (except the export modal). The goal is to shift to an Arc-browser-inspired aesthetic: mostly opaque surfaces with soft shadows for depth, minimal borders, and a clean "floating" feel. Overlays get slightly more translucency and stronger shadows than main content surfaces.

This is **not** heavy frosted-glass glassmorphism. It's Arc-faithful: solid, grounded surfaces with just enough translucency on overlays to feel alive. The floating feel comes from elevation (shadows), not transparency.

## Elevation System

Three tiers replace the current "1px border everywhere" approach:

| Tier | Name | Shadow | Use |
|------|------|--------|-----|
| 0 | Flush | None | Input fields, banners — recessed by darker fill |
| 1 | Resting | `offset [0,1], blur 3, spread 0, rgba(0,0,0,64)` | Cards, top bar, footer, primary/secondary buttons |
| 2 | Floating | `offset [0,4], blur 16, spread 0, rgba(0,0,0,89)` | History panel, command palette, export modal, batch panel |

Footer uses an upward variant of Level 1: `offset [0,-1], blur 3, spread 0, rgba(0,0,0,64)`.

Light mode uses softer shadow opacities: Level 1 `rgba(0,0,0,25)`, Level 2 `rgba(0,0,0,40)`.

**Note:** egui 0.31 `Shadow` fields are `offset: [i8; 2]`, `blur: u8`, `spread: u8`, `color: Color32`. Spread is unsigned — no negative values. All values above fit these constraints.

### Border Policy

- **Removed** from: cards, inputs, buttons, panels, overlays, top bar, footer
- **Kept** for: focus rings (`border_focus` blue), accent banner strokes (semantic — communicates status), explicit section dividers (where two same-colored surfaces meet)
- `border_subtle` / `border_default` tokens remain in the palette for divider use

### Corner Radius

No changes. Current hierarchy is already Arc-appropriate:
- 12px: windows, overlay panels
- 8px: cards
- 6px: inputs, buttons
- 4px: chips, small pills

## Surface Treatment

### Main Surfaces (always-visible, opaque, grounded)

| Surface | Fill | Elevation | Notes |
|---------|------|-----------|-------|
| App background | `bg_base` (#0D0F12) | None | Unchanged |
| Top bar | `bg_surface` (#14161B) | Level 1 | Remove bottom border, add shadow |
| Footer | `bg_surface` (#14161B) | Level 1 (upward) | Remove top border, add shadow_up |
| Cards (output, JWT, EXIF) | `bg_card` (#1E2128) | Level 1 | Remove stroke |
| Input field | `bg_input` (#12141A) | Level 0 | No stroke, no shadow — recessed by being darker |
| Primary/Secondary buttons | Existing fills | Level 1 | Remove stroke |
| Ghost buttons | Transparent | Level 0 | Unchanged |
| Accent banners | Dim fill + accent stroke | Level 0 | Unchanged — strokes are semantic |

### Overlays (slightly translucent, elevated)

| Overlay | Fill | Alpha | Elevation | Notes |
|---------|------|-------|-----------|-------|
| History panel | `panel_glass` | 0xEB (92%) | Level 2 | Bumped from 87% to 92% |
| Command palette | New `overlay_surface` | 0xF0 (94%) | Level 2 | Currently uses default egui frame |
| Export modal | `overlay_surface` | 0xF0 (94%) | Level 2 | Drop border, upgrade shadow |
| Batch panel | `panel_glass` | 0xEB (92%) | Level 2 | Currently solid |
| Modal backdrop | `modal_backdrop` | 60% | None | Unchanged |

## New Tokens

Add to `Tokens` struct in `src/theme.rs`:

```
shadow_sm     // Level 1 — resting elevation
shadow_lg     // Level 2 — floating elevation
shadow_up     // Level 1 upward — footer
overlay_surface  // modal_surface with 94% alpha for overlays
```

Update existing:
- `panel_glass` alpha: 0xDD → 0xEB (dark), 0xDD → 0xEB (light)

## Files to Modify

| File | Changes |
|------|---------|
| `src/theme.rs` | Add `shadow_sm`, `shadow_lg`, `shadow_up`, `overlay_surface` to `Tokens`. Update `panel_glass` alpha. Update `apply()`: set `window_shadow` to `shadow_sm`, remove default widget strokes from noninteractive/inactive/hovered/open states, keep active stroke for focus rings. |
| `src/ui/widgets.rs` | `card_frame()`: add `shadow_sm`, remove stroke. `input_frame()`: remove stroke, no shadow. `glass_panel()`: use updated `panel_glass`, add `shadow_lg`. New `overlay_frame()` helper: `overlay_surface` fill + `shadow_lg` + 12px radius. |
| `src/ui/top_bar.rs` | Remove manual bottom border painting (the `foreground` rect_filled line). Frame gets shadow from Level 1. |
| `src/ui/output.rs` | Image preview frame + metadata bar: remove strokes, add `shadow_sm`. |
| `src/ui/export_image_dialog.rs` | Use `overlay_frame()` for modal window. Drop `border_default` stroke. Use `shadow_lg`. |
| `src/ui/command_palette.rs` | Use `overlay_frame()` with `shadow_lg`. Remove default window frame styling. |
| `src/ui/history_panel.rs` | `glass_panel()` already updated via widgets.rs. Verify shadow_lg applies. |
| `src/ui/batch_panel.rs` | Switch from solid panel to `glass_panel()` style with `shadow_lg`. |
| `src/ui/buttons.rs` | Remove border strokes from primary/secondary buttons. Add `shadow_sm` to painted button rects. |
| `src/app.rs` | Footer frame: remove top border stroke, add `shadow_up`. |

### Files NOT changing

- `src/core/*` — pure logic, no UI
- `src/cli.rs`, `src/settings.rs`, `src/samples.rs`
- `src/decode.rs`, `src/detect.rs` — state adapters only
- `src/ui/banner.rs` — accent strokes are semantic, kept as-is
- `src/ui/diff_view.rs` — uses hardcoded diff colors (green/red fills), not structural borders

## Verification

1. `cargo build` — compiles without error
2. `cargo clippy --all-targets -- -D warnings` — clean
3. `cargo run` — launch GUI, visually verify:
   - Top bar floats with subtle shadow, no bottom line
   - Cards have soft shadow, no borders
   - Input fields are recessed (darker fill, no border)
   - Primary/secondary buttons cast subtle shadow
   - Banners still have accent-colored strokes
   - Open history panel: slightly translucent, strong shadow, floats above content
   - Open command palette: same floating overlay treatment
   - Open export modal: translucent surface, floating shadow, no border
   - Toggle light mode: shadows are softer, same hierarchy
   - Focus an input: blue focus ring still visible
4. `cargo test` — all tests pass (no UI logic changes)
