/// Command registry for the command palette.
///
/// Pure Rust module — no egui imports. Commands are defined as data.
/// The UI layer (`ui/command_palette.rs`) executes commands by matching
/// on `Command::id` strings; this module owns only the registry and
/// fuzzy-search logic.

/// A registered command in the palette.
pub struct Command {
    /// Unique stable identifier used for dispatch and debugging.
    pub id: &'static str,
    /// Human-readable name shown in the palette.
    pub name: &'static str,
    /// Space-separated keywords for fuzzy matching.
    pub keywords: &'static str,
    /// Shortcut hint shown on the right side (e.g. "⌘Enter").
    pub shortcut_display: &'static str,
}

/// All built-in commands. Order determines the default display order
/// when the palette is opened with an empty query.
pub const COMMANDS: &[Command] = &[
    Command {
        id: "encode",
        name: "Encode → Base64",
        keywords: "encode base64 convert",
        shortcut_display: "⌘Enter",
    },
    Command {
        id: "decode",
        name: "Decode → Text / Image",
        keywords: "decode text image",
        shortcut_display: "⌘Enter",
    },
    Command {
        id: "copy_output",
        name: "Copy Output",
        keywords: "copy clipboard output",
        shortcut_display: "⌘⇧C",
    },
    Command {
        id: "clear_all",
        name: "Clear All",
        keywords: "clear reset clean escape",
        shortcut_display: "Esc",
    },
    Command {
        id: "toggle_theme",
        name: "Toggle Theme",
        keywords: "theme cycle light dark system",
        shortcut_display: "",
    },
    Command {
        id: "theme_light",
        name: "Theme: Light",
        keywords: "theme light sun bright",
        shortcut_display: "",
    },
    Command {
        id: "theme_dark",
        name: "Theme: Dark",
        keywords: "theme dark moon night",
        shortcut_display: "",
    },
    Command {
        id: "theme_system",
        name: "Theme: System",
        keywords: "theme system os auto detect",
        shortcut_display: "",
    },
    Command {
        id: "open_history",
        name: "Open History",
        keywords: "history recent entries panel",
        shortcut_display: "⌘H",
    },
    Command {
        id: "toggle_private",
        name: "Toggle Private Mode",
        keywords: "private privacy mode incognito",
        shortcut_display: "",
    },
    Command {
        id: "batch_encode_folder",
        name: "Batch Encode Folder…",
        keywords: "batch encode folder directory bulk",
        shortcut_display: "",
    },
    Command {
        id: "batch_decode_folder",
        name: "Batch Decode Folder…",
        keywords: "batch decode folder directory bulk",
        shortcut_display: "",
    },
    Command {
        id: "show_diff_mode",
        name: "Show Diff Mode",
        keywords: "diff compare comparison",
        shortcut_display: "⌘D",
    },
    Command {
        id: "copy_md5",
        name: "Copy MD5",
        keywords: "md5 hash checksum digest",
        shortcut_display: "",
    },
    Command {
        id: "copy_sha256",
        name: "Copy SHA-256",
        keywords: "sha256 sha hash checksum digest",
        shortcut_display: "",
    },
    Command {
        id: "export_image",
        name: "Export Image",
        keywords: "export image save download file",
        shortcut_display: "",
    },
];

// ---------------------------------------------------------------------------
// Fuzzy matching
// ---------------------------------------------------------------------------

/// Returns `true` if `query` is a subsequence of `text` (case-insensitive).
///
/// A subsequence match means all characters in `query` appear in `text`
/// in the same order, but not necessarily consecutively.
///
/// Examples:
/// - `"cpy"` matches `"Copy Output"` ✓
/// - `"dc"`  matches `"Decode"` ✓
/// - `"xyz"` matches `"Clear All"` ✗
#[cfg(test)]
fn is_subsequence_match(query: &str, text: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    let query_lower = query.to_lowercase();
    let text_lower = text.to_lowercase();
    let mut query_chars = query_lower.chars().peekable();
    for c in text_lower.chars() {
        if let Some(&q) = query_chars.peek() {
            if c == q {
                query_chars.next();
            }
        } else {
            break;
        }
    }
    query_chars.peek().is_none()
}

/// Score a fuzzy match. Lower is better (tighter fit).
/// Returns `None` if the query does not match.
///
/// Score = index of first matched char + index of last matched char + gaps.
/// This prefers matches where query characters appear early and close together.
pub fn fuzzy_score(query: &str, text: &str) -> Option<u32> {
    if query.is_empty() {
        return Some(0);
    }

    let query_lower: Vec<char> = query.to_lowercase().chars().collect();
    let text_lower: Vec<char> = text.to_lowercase().chars().collect();

    let mut qi = 0;
    let mut first_match: Option<u32> = None;
    let mut last_match: Option<u32> = None;

    for (ti, &tc) in text_lower.iter().enumerate() {
        if qi < query_lower.len() && tc == query_lower[qi] {
            if first_match.is_none() {
                first_match = Some(ti as u32);
            }
            last_match = Some(ti as u32);
            qi += 1;
        }
    }

    if qi < query_lower.len() {
        return None; // Not all query chars matched
    }

    let first = first_match.unwrap_or(0);
    let last = last_match.unwrap_or(0);
    let span = last - first + 1;
    let density = query_lower.len() as u32;

    // Score: earlier match + denser = better
    Some(first + span - density)
}

/// Filter and score commands by query.
/// Returns sorted by score (best first), then by original order for ties.
///
/// Uses `sort_by_key` (stable) intentionally — an empty query scores all
/// commands at 0, and stable sort preserves the original `COMMANDS` order.
/// Do not replace with `sort_unstable_by_key`.
pub fn filter_commands(query: &str) -> Vec<(usize, &Command, u32)> {
    let query = query.trim();
    let mut results: Vec<(usize, &Command, u32)> = COMMANDS
        .iter()
        .enumerate()
        .filter_map(|(idx, cmd)| {
            // Match against name and keywords
            let name_match = fuzzy_score(query, cmd.name);
            let keyword_match = fuzzy_score(query, cmd.keywords);

            let best_score = match (name_match, keyword_match) {
                (Some(ns), Some(ks)) => Some(ns.min(ks + 5)), // keyword matches penalized slightly
                (Some(ns), None) => Some(ns),
                (None, Some(ks)) => Some(ks + 5),
                (None, None) => None,
            };

            best_score.map(|score| (idx, cmd, score))
        })
        .collect();

    results.sort_by_key(|&(_, _, score)| score);
    results
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Fuzzy matching tests --

    #[test]
    fn subsequence_cpy_matches_copy_output() {
        assert!(is_subsequence_match("cpy", "Copy Output"));
    }

    #[test]
    fn subsequence_dc_matches_decode() {
        assert!(is_subsequence_match("dc", "Decode"));
    }

    #[test]
    fn subsequence_xyz_matches_nothing() {
        assert!(!is_subsequence_match("xyz", "Clear All"));
        assert!(!is_subsequence_match("xyz", "Copy Output"));
        assert!(!is_subsequence_match("xyz", "Encode"));
    }

    #[test]
    fn empty_query_matches_everything() {
        assert!(is_subsequence_match("", "Anything"));
        assert!(is_subsequence_match("", ""));
    }

    #[test]
    fn subsequence_case_insensitive() {
        assert!(is_subsequence_match("ENCODE", "Encode"));
        assert!(is_subsequence_match("encode", "ENCODE"));
        assert!(is_subsequence_match("EnCoDe", "eNcOdE"));
    }

    #[test]
    fn fuzzy_score_cpy_on_copy_output() {
        let score = fuzzy_score("cpy", "Copy Output");
        assert!(score.is_some());
        // c=0, p=2, y=3 → first=0, last=3, span=4, density=3 → 0+4-3=1
        assert_eq!(score, Some(1));
    }

    #[test]
    fn fuzzy_score_empty_query() {
        assert_eq!(fuzzy_score("", "Copy Output"), Some(0));
    }

    #[test]
    fn fuzzy_score_no_match() {
        assert!(fuzzy_score("xyz", "Clear All").is_none());
    }

    #[test]
    fn filter_commands_empty_query_returns_all() {
        let results = filter_commands("");
        assert_eq!(results.len(), COMMANDS.len());
        // Should preserve original order when scores are tied at 0
        assert_eq!(results[0].0, 0); // encode is first
    }

    #[test]
    fn filter_commands_cpy_finds_copy() {
        let results = filter_commands("cpy");
        assert!(!results.is_empty());
        // "Copy Output" should be near the top
        let copy_cmd = results.iter().find(|(_, cmd, _)| cmd.id == "copy_output");
        assert!(copy_cmd.is_some());
    }

    #[test]
    fn filter_commands_theme_finds_theme() {
        let results = filter_commands("theme");
        // Should find all theme-related commands
        assert!(results.len() >= 4); // toggle_theme + light + dark + system
        assert!(results.iter().any(|(_, cmd, _)| cmd.id == "toggle_theme"));
        assert!(results.iter().any(|(_, cmd, _)| cmd.id == "theme_light"));
        assert!(results.iter().any(|(_, cmd, _)| cmd.id == "theme_dark"));
    }

    #[test]
    fn filter_commands_batch_finds_batch() {
        let results = filter_commands("batch");
        assert!(results.len() >= 2);
        assert!(results
            .iter()
            .any(|(_, cmd, _)| cmd.id.starts_with("batch_")));
    }

    #[test]
    fn filter_commands_hash_finds_md5_and_sha() {
        let results = filter_commands("hash");
        assert!(results.iter().any(|(_, cmd, _)| cmd.id == "copy_md5"));
        assert!(results.iter().any(|(_, cmd, _)| cmd.id == "copy_sha256"));
    }
}
