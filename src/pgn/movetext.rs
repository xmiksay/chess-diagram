//! PGN text → header tag pairs and the mainline's SAN tokens. A
//! character-level lexer rather than a word/line splitter, because PGN's
//! delimiters are self-terminating: a comment may be glued to a move
//! (`e4{best}`), span lines that start with `[` (`{ ...\n[%clk 0:03:00] }`),
//! or contain `)`, `{` and `;` literally.

use std::borrow::Cow;

/// One lexed PGN token: a header tag pair or a mainline SAN move.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Token<'a> {
    Tag(Tag<'a>),
    San(&'a str),
}

impl<'a> Token<'a> {
    pub(super) fn san(self) -> Option<&'a str> {
        match self {
            Token::San(san) => Some(san),
            Token::Tag(_) => None,
        }
    }
}

/// A `[Name "value"]` header tag pair. `value` is still escaped (see
/// [`unescape`]); a malformed tag keeps whatever followed its opening quote.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Tag<'a> {
    pub(super) name: &'a str,
    pub(super) value: &'a str,
}

/// Resolves a tag value's `\"` and `\\` escapes, borrowing when it has none.
pub(super) fn unescape(value: &str) -> Cow<'_, str> {
    if !value.contains('\\') {
        return Cow::Borrowed(value);
    }
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(c) = chars.next() {
        out.push(match c {
            '\\' => chars.next().unwrap_or(c),
            _ => c,
        });
    }
    Cow::Owned(out)
}

/// Yields a PGN's header tag pairs, then its mainline's SAN move tokens,
/// skipping move numbers, `{...}` and `;` comments, `%` escape lines,
/// variations, NAGs, and `!`/`?` suffix annotations. A game-result token
/// (`1-0`, `0-1`, `1/2-1/2`, `*`) ends the mainline, and so does a tag pair
/// once moves have started (a result-less game followed by the next game's
/// headers), so a second concatenated game is never played onto the first
/// one's board — nor are its tags read as the first one's.
pub(super) fn tokens(pgn: &str) -> impl Iterator<Item = Token<'_>> {
    // Editors on Windows prepend a BOM, which isn't whitespace to `trim_start`.
    let pgn = pgn.strip_prefix('\u{feff}').unwrap_or(pgn);
    let mut rest = pgn;
    let mut variation_depth = 0u32;
    let mut in_movetext = false;
    std::iter::from_fn(move || loop {
        let trimmed = rest.trim_start();
        let mut chars = trimmed.chars();
        let Some(first) = chars.next() else {
            rest = "";
            return None;
        };
        let after = chars.as_str();
        rest = match first {
            // `%` escapes a line only in column 0.
            '%' if pgn
                .get(..pgn.len() - trimmed.len())
                .is_some_and(|p| p.is_empty() || p.ends_with('\n')) =>
            {
                skip_line(after)
            }
            ';' => skip_line(after),
            // Braces don't nest: a `{` inside a comment is literal text.
            '{' => after.split_once('}').map_or("", |(_, tail)| tail),
            // Tag pairs only precede movetext, so one after a move is the
            // next game's header section even if this game lost its result.
            '[' if variation_depth == 0 && in_movetext => {
                rest = "";
                return None;
            }
            '[' if variation_depth == 0 => {
                let (tag, tail) = tag(after);
                rest = tail;
                return Some(Token::Tag(tag));
            }
            '(' => {
                variation_depth = variation_depth.saturating_add(1);
                after
            }
            ')' => {
                variation_depth = variation_depth.saturating_sub(1);
                after
            }
            // A NAG is `$<digits>`; a malformed one is still dropped whole.
            '$' => after.trim_start_matches(|c: char| !is_delimiter(c)),
            // Import-format suffix annotations (`e4!`, `Nf3?!`), plus a check
            // marker written after one (`Bb5?+`) — the SAN already ended.
            '!' | '?' => after.trim_start_matches(['!', '?', '+', '#']),
            _ => {
                let end = trimmed
                    .char_indices()
                    .skip(1)
                    .find(|&(_, c)| is_delimiter(c))
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
                // leading digit of zero-castling (`0-0`).
                let unnumbered = symbol.trim_start_matches(|c: char| c.is_ascii_digit());
                let san = if unnumbered.is_empty() || unnumbered.starts_with('.') {
                    unnumbered.trim_start_matches('.')
                } else {
                    symbol
                };
                if san.is_empty() {
                    continue;
                }
                in_movetext = true;
                return Some(Token::San(san));
            }
        };
    })
}

/// Ends a move symbol: whitespace or a self-terminating PGN token start.
fn is_delimiter(c: char) -> bool {
    c.is_whitespace() || "{}();[$!?".contains(c)
}

fn skip_line(s: &str) -> &str {
    s.split_once('\n').map_or("", |(_, tail)| tail)
}

/// Lexes a `[Name "value"]` tag pair (after its `[`) up to the `]` outside
/// the quoted value, or the end of the line if the tag is malformed.
fn tag(s: &str) -> (Tag<'_>, &str) {
    // Byte offsets of the first two unescaped quotes: the value's delimiters.
    let mut quotes = [None, None];
    let mut quote_count = 0usize;
    let mut escaped = false;
    let (end, rest) = 'scan: {
        for (i, c) in s.char_indices() {
            let in_string = quote_count % 2 == 1;
            match c {
                _ if escaped => escaped = false,
                '\\' if in_string => escaped = true,
                '"' => {
                    if let Some(slot) = quotes.get_mut(quote_count) {
                        *slot = Some(i);
                    }
                    quote_count += 1;
                }
                ']' if !in_string => break 'scan (i, s.get(i + 1..).unwrap_or("")),
                '\n' => break 'scan (i, s.get(i..).unwrap_or("")),
                _ => {}
            }
        }
        (s.len(), "")
    };
    let [open, close] = quotes;
    let name = s.get(..open.unwrap_or(end)).unwrap_or("").trim();
    let value = open
        .and_then(|open| s.get(open + 1..close.unwrap_or(end)))
        .unwrap_or("");
    (Tag { name, value }, rest)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sans(pgn: &str) -> Vec<&str> {
        tokens(pgn).filter_map(Token::san).collect()
    }

    fn tags(pgn: &str) -> Vec<(&str, &str)> {
        tokens(pgn)
            .filter_map(|t| match t {
                Token::Tag(tag) => Some((tag.name, tag.value)),
                Token::San(_) => None,
            })
            .collect()
    }

    #[test]
    fn tag_pairs_yield_name_and_raw_value() {
        assert_eq!(
            tags(
                r#"[Event "a ] b"]
[Site"x\"y\\"]  [ FEN  "8/8 w" ]
1. e4 [After "x"]"#
            ),
            [("Event", "a ] b"), ("Site", r#"x\"y\\"#), ("FEN", "8/8 w")]
        );
        // Malformed tags keep what they can, up to the end of their line.
        assert_eq!(
            tags("[FEN \"8/8 w\n[SetUp]\n[Event \"x\" junk]"),
            [("FEN", "8/8 w"), ("SetUp", ""), ("Event", "x")]
        );
    }

    #[test]
    fn unescape_resolves_quote_and_backslash() {
        assert!(matches!(unescape("plain"), Cow::Borrowed("plain")));
        assert_eq!(unescape(r#"x\"y\\z"#), r#"x"y\z"#);
        assert_eq!(unescape("trailing\\"), "trailing\\");
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
    fn tag_pair_after_moves_ends_mainline() {
        // The first game is missing its result token.
        assert_eq!(
            sans("[Event \"a\"]\n1. e4 e5\n\n[Event \"b\"]\n1. d4 d5"),
            ["e4", "e5"]
        );
        // Inside a variation a `[` is not a header.
        assert_eq!(sans("1. e4 (1. d4 [x]) e5"), ["e4", "e5"]);
    }

    #[test]
    fn stays_ended_after_result() {
        let mut it = tokens("1. e4 1-0 2. d4");
        assert_eq!(it.next(), Some(Token::San("e4")));
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
            sans("1. e4! e5?! 2. Nf3!! Nc6 ? 3. Bb5+? a6 4. Ba4?+ b5"),
            ["e4", "e5", "Nf3", "Nc6", "Bb5+", "a6", "Ba4", "b5"]
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
        // A move number glued to zero-castling strips only the number.
        assert_eq!(sans("4.0-0 4...0-0-0"), ["0-0", "0-0-0"]);
    }

    #[test]
    fn malformed_nag_is_dropped_whole() {
        assert_eq!(sans("1. e4 $x e5 $12a{c} Nf3"), ["e4", "e5", "Nf3"]);
    }

    #[test]
    fn nags_and_nested_variations_are_skipped() {
        assert_eq!(
            sans("1. e4 $1 e5$2 (1... c5 (1... e6) 2. Nf3) 2. Nf3"),
            ["e4", "e5", "Nf3"]
        );
    }
}
