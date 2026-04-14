# Embedded fonts

These TTFs are compiled into `basie-64` via `include_bytes!` in `src/theme.rs` so
the binary renders identically on any machine, online or offline.

| File | Family | Source | License |
|---|---|---|---|
| `Inter-Regular.ttf` | Inter (regular) | [rsms/inter v4.1](https://github.com/rsms/inter/releases/tag/v4.1) — `extras/ttf/` | SIL OFL 1.1 (`LICENSE-INTER.txt`) |
| `Inter-SemiBold.ttf` | Inter (semibold) | [rsms/inter v4.1](https://github.com/rsms/inter/releases/tag/v4.1) — `extras/ttf/` | SIL OFL 1.1 (`LICENSE-INTER.txt`) |
| `IBMPlexMono-Regular.ttf` | IBM Plex Mono | [IBM/plex @ibm/plex-mono@1.1.0](https://github.com/IBM/plex/releases/tag/%40ibm%2Fplex-mono%401.1.0) — `fonts/complete/ttf/` | SIL OFL 1.1 (`LICENSE-IBM-PLEX.txt`) |
| `lucide.ttf` | Lucide icon font | [lucide-icons/lucide font 1.8.0](https://github.com/lucide-icons/lucide/releases/tag/1.8.0) — `lucide-font-1.8.0.zip` | ISC (`LICENSE-LUCIDE.txt`) |
| `lucide-codepoints.json` | Full lucide `info.json` | shipped inside `lucide-font-1.8.0.zip` | ISC |

## Regenerating icon codepoints

`src/theme.rs` only hard-codes a small subset of glyphs (~20). To add or bump a
glyph, look it up in `lucide-codepoints.json` by its kebab-case name — the
`encodedCode` field is `\eHHHH` where `HHHH` is the hex codepoint in the
Private Use Area. Add a matching `pub const` in `theme::icons`.

When bumping the Lucide release, re-download the font zip, overwrite
`lucide.ttf` + `lucide-codepoints.json`, and re-verify every constant in
`theme::icons` is still present (codepoints are stable across Lucide releases,
but verify).
