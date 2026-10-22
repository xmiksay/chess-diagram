//! PGN text → the mainline's SAN tokens. A character-level lexer rather than a
//! word/line splitter, because PGN's delimiters are self-terminating: a
//! comment may be glued to a move (`e4{best}`), span lines that start with
//! `[` (`{ ...\n[%clk 0:03:00] }`), or contain `)`, `{` and `;` literally.

/// Yields SAN move tokens from a PGN's mainline, skipping tag pairs, move
/// numbers, `{...}` and `;` comments, `%` escape lines, variations, NAGs, and
/// `!`/`?` suffix annotations. A game-result token (`1-0`, `0-1`, `1/2-1/2`,
/// `*`) ends the mainline, so a second concatenated game is never played onto
/// the first one's board.
pub(super) fn mainline_sans(pgn: &str) -> impl Iterator<Item = &str> {
    // Editors on Windows prepend a BOM, which isn't whitespace to `trim_start`.
    let pgn = pgn.strip_prefix('\u{feff}').unwrap_or(pgn);
    let mut rest = pgn;
    let mut variation_depth = 0u32;
    std::iter::from_fn(move || loop {
        let trimmed = rest.trim_start();
        // `%` escapes a line only in column 0.
        let offset = pgn.len() - trimmed.len();
        let line_start = offset == 0 || pgn.get(..offset).is_some_and(|p| p.ends_with('\n'));
        let mut chars = trimmed.chars();
        let Some(first) = chars.next() else {
            rest = "";
            return None;
        };
        let after = chars.as_str();
        rest = match first {
            '%' if line_start => skip_line(after),
            ';' => skip_line(after),
            // Braces don't nest: a `{` inside a comment is literal text.
            '{' => after.split_once('}').map_or("", |(_, tail)| tail),
            '[' if variation_depth == 0 => skip_tag(after),
            '(' => {
                variation_depth = variation_depth.saturating_add(1);
                after
            }
            ')' => {
                variation_depth = variation_depth.saturating_sub(1);
                after
            }
            '$' => after.trim_start_matches(|c: char| c.is_ascii_digit()),
            // Import-format suffix annotations (`e4!`, `Nf3?!`).
            '!' | '?' => after.trim_start_matches(['!', '?']),
            _ => {
                let end = trimmed
                    .char_indices()
                    .skip(1)
                    .find(|&(_, c)| c.is_whitespace() || "{}();[$!?".contains(c))
                    .map_or(trimmed.len(), |(i, _)| i);
                let (symbol, tail) = trimmed.split_at_checked(end)?;
                rest = tail;
                if variation_depth > 0 {
                    continue;
                }
                if matches!(symbol, "1-0" | "0-1" | "1/2-1/2" | "*") {
                    rest = "";
                    return None;
                }
                // Strip a move number (`12.`, `12...`, bare `12`) but not the
                // digits of a non-SAN token like `0-0`, so its error names it whole.
                let unnumbered = symbol.trim_start_matches(|c: char| c.is_ascii_digit());
                let san = if unnumbered.is_empty() || unnumbered.starts_with('.') {
                    unnumbered.trim_start_matches('.')
                } else {
                    symbol
                };
                if san.is_empty() {
                    continue;
                }
                return Some(san);
            }
        };
    })
}

fn skip_line(s: &str) -> &str {
    s.split_once('\n').map_or("", |(_, tail)| tail)
}

/// Skips a `[Name "value"]` tag pair (after its `[`): up to the `]` outside
/// the quoted value, or the end of the line if the tag is malformed.
fn skip_tag(s: &str) -> &str {
    let mut in_string = false;
    let mut escaped = false;
    for (i, c) in s.char_indices() {
        match c {
            _ if escaped => escaped = false,
            '\\' if in_string => escaped = true,
            '"' => in_string = !in_string,
            ']' if !in_string => return s.get(i + 1..).unwrap_or(""),
            '\n' => return s.get(i..).unwrap_or(""),
            _ => {}
        }
    }
    ""
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sans(pgn: &str) -> Vec<&str> {
        mainline_sans(pgn).collect()
    }

    #[test]
    fn result_token_ends_mainline() {
        assert_eq!(
            sans("1. e4 e5 2. Qh5 Nc6 3. Bc4 Nf6 4. Qxf7# 1-0 1. d4").len(),
            7
        );
        // Inside a comment or variation a result-looking token doesn't end it.
        assert_eq!(
            sans("1. e4 { 1-0 } e5 (1... c5 *) 2. Nf3"),
            ["e4", "e5", "Nf3"]
        );
    }

    #[test]
    fn stays_ended_after_result() {
        let mut it = mainline_sans("1. e4 1-0 2. d4");
        assert_eq!(it.next(), Some("e4"));
        assert_eq!(it.next(), None);
        assert_eq!(it.next(), None);
    }

    #[test]
    fn tags_are_skipped_even_with_brackets_in_values() {
        assert_eq!(
            sans("[Event \"a ] b\"] [Site \"x\\\"y\"]\n1. e4 e5"),
            ["e4", "e5"]
        );
        // A malformed tag only costs its own line.
        assert_eq!(sans("[Event \"oops\n1. e4"), ["e4"]);
    }

    #[test]
    fn multi_line_comment_with_bracket_line_closes() {
        assert_eq!(
            sans("1. e4 { long\n[%clk 0:03:00] } e5 2. Nf3"),
            ["e4", "e5", "Nf3"]
        );
    }

    #[test]
    fn comments_glued_to_moves() {
        assert_eq!(
            sans("1. e4{best}e5 2.Nf3{a b}Nc6"),
            ["e4", "e5", "Nf3", "Nc6"]
        );
    }

    #[test]
    fn braces_do_not_nest() {
        assert_eq!(sans("1. e4 { see {x } e5"), ["e4", "e5"]);
    }

    #[test]
    fn rest_of_line_and_escape_comments() {
        assert_eq!(
            sans("% exported by X\n1. e4 ; best by test }\n% escape after a ; comment\ne5 (1... c5 ; :-)\n) 2. Nf3"),
            ["e4", "e5", "Nf3"]
        );
        // `%` is only an escape in column 0.
        assert_eq!(sans("1. e4 %x"), ["e4", "%x"]);
    }

    #[test]
    fn suffix_annotations_are_skipped() {
        assert_eq!(
            sans("1. e4! e5?! 2. Nf3!! Nc6 ? 3. Bb5+?"),
            ["e4", "e5", "Nf3", "Nc6", "Bb5+"]
        );
    }

    #[test]
    fn leading_bom_is_ignored() {
        assert_eq!(sans("\u{feff}[Event \"x\"]\n1. e4"), ["e4"]);
        assert_eq!(sans("\u{feff}% escape\n1. e4"), ["e4"]);
    }

    #[test]
    fn move_numbers_strip_but_zero_castling_stays_whole() {
        assert_eq!(sans("1.e4 1...e5 2 Nf3 0-0"), ["e4", "e5", "Nf3", "0-0"]);
    }

    #[test]
    fn nags_and_nested_variations_are_skipped() {
        assert_eq!(
            sans("1. e4 $1 e5$2 (1... c5 (1... e6) 2. Nf3) 2. Nf3"),
            ["e4", "e5", "Nf3"]
        );
    }
}
