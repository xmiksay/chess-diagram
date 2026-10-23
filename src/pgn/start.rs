//! The position a PGN mainline starts from: its `FEN` and `Variant` header
//! tags → a `cozy_chess` board. Chess960 and standard chess share cozy-chess's
//! move generator (castling is always "king takes own rook"), so the variant
//! only changes which castling rights a FEN may grant.
//!
//! The FEN is read leniently where that can't change the board, as
//! python-chess and lichess do: unsupportable castling rights (see
//! `castling.rs`) and an en passant square no pawn could capture on are
//! dropped rather than failing the diagram.

use cozy_chess::{Board, Square};

use super::castling::{implied_variant, with_castling};
use super::PgnError;

/// `Variant` tag spellings played with standard move rules, compared ignoring
/// case, whitespace, `-` and `_` (lichess writes `Standard` and `From Position`;
/// the rest follow python-chess's aliases). Three-check and King of the Hill
/// only change how a game is won, never which moves are legal, so their
/// boards are walked like standard chess.
const STANDARD: &[&str] = &[
    "standard",
    "chess",
    "normal",
    "classical",
    "fromposition",
    "threecheck",
    "3check",
    "kingofthehill",
    "koth",
];
const CHESS960: &[&str] = &["chess960", "fischerandom", "fischerrandom"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Variant {
    Standard,
    Chess960,
}

/// The board a mainline starts from: `fen` (the unescaped `FEN` tag) when
/// present, else the standard start position (also Chess960 position 518).
/// An empty FEN tag is an exporter placeholder, like an empty `Variant`.
pub(super) fn start_position(fen: Option<&str>, variant: Option<&str>) -> Result<Board, PgnError> {
    let variant = parse_variant(variant)?;
    match fen.filter(|f| !f.trim().is_empty()) {
        None => Ok(Board::startpos()),
        Some(fen) => parse_fen(fen, variant).ok_or_else(|| PgnError::InvalidFen(fen.to_string())),
    }
}

/// `None` when the tag is absent or empty (some exporters write one), which
/// leaves the variant to the FEN's castling field.
fn parse_variant(tag: Option<&str>) -> Result<Option<Variant>, PgnError> {
    let Some(tag) = tag.filter(|t| !t.trim().is_empty()) else {
        return Ok(None);
    };
    let key: String = tag
        .chars()
        .filter(|c| !(c.is_whitespace() || matches!(c, '-' | '_')))
        .map(|c| c.to_ascii_lowercase())
        .collect();
    let is_one_of = |names: &[&str]| names.contains(&key.as_str());
    if is_one_of(STANDARD) {
        Ok(Some(Variant::Standard))
    } else if is_one_of(CHESS960) {
        Ok(Some(Variant::Chess960))
    } else {
        Err(PgnError::UnsupportedVariant(tag.to_string()))
    }
}

/// Parses a FEN, its castling field via [`with_castling`]. Without an explicit
/// variant, the castling field decides (see [`implied_variant`]).
fn parse_fen(fen: &str, variant: Option<Variant>) -> Option<Board> {
    let mut fields = fen.split_ascii_whitespace();
    let (placement, side, castling, en_passant) = (
        fields.next()?,
        fields.next()?,
        fields.next()?,
        fields.next()?,
    );
    // Three-check's checks-remaining field (`3+3`, lichess's `+0+0`) never changes the board.
    let mut fields = fields.filter(|f| {
        let counts = f.strip_prefix('+').unwrap_or(f).split_once('+');
        !counts.is_some_and(|(w, b)| w.parse::<u8>().is_ok() && b.parse::<u8>().is_ok())
    });
    // Clocks are optional (EPD-style FENs omit them) and never change the
    // board, so they're only checked to be numbers: cozy would also reject a
    // halfmove clock over 100, which a legal (unclaimed) position can have.
    for clock in fields.by_ref().take(2) {
        clock.parse::<u32>().ok()?;
    }
    // cozy accepts a placement with fewer than 8 ranks as a shorter board.
    if fields.next().is_some() || placement.split('/').count() != 8 {
        return None;
    }
    if en_passant != "-" && en_passant.parse::<Square>().is_err() {
        return None;
    }
    // cozy rejects an en passant square without a capturable pawn behind it;
    // such a square can never enable a capture, so it's dropped instead.
    let board = Board::from_fen(&format!("{placement} {side} - {en_passant} 0 1"), false)
        .or_else(|_| Board::from_fen(&format!("{placement} {side} - - 0 1"), false))
        .ok()?;

    let variant = variant.unwrap_or_else(|| implied_variant(&board, castling));
    with_castling(&board, castling, variant)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pgn::board_at;

    const START: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w";
    /// A Chess960 start (king f1, rooks e1/g1) and the board after both sides `O-O`.
    const C960_START: &str = "bqnbrkrn/pppppppp/8/8/8/8/PPPPPPPP/BQNBRKRN";
    const C960_CASTLED: &str = "bqnbrrkn/pppppppp/8/8/8/8/PPPPPPPP/BQNBRRKN w";

    /// `board_at(pgn, ply)` equals the position `expected` (placement + side).
    fn assert_board(pgn: &str, ply: usize, expected: &str) {
        let expected = crate::fen::parse(expected).expect("valid expected FEN");
        assert_eq!(board_at(pgn, ply), Ok(expected), "{pgn} @ {ply}");
    }

    #[test]
    fn fen_tag_sets_the_start_position() {
        let pgn = "[Event \"Puzzle\"]\n[SetUp \"1\"]\n[FEN \"6k1/5ppp/8/8/8/8/8/R5K1 w - - 0 1\"]\n\n1. Ra8# 1-0";
        assert_board(pgn, 0, "6k1/5ppp/8/8/8/8/8/R5K1 w");
        assert_board(pgn, 1, "R5k1/5ppp/8/8/8/8/8/6K1 b");
    }

    #[test]
    fn fen_tag_wins_when_moves_are_legal_from_both_positions() {
        // `1. e4` is also legal from the start position; the FEN must win,
        // with or without `SetUp`.
        let fen = "8/8/8/8/8/8/4P3/4K2k w - - 0 1";
        for header in [
            format!("[FEN \"{fen}\"]"),
            format!("[SetUp \"1\"]\n[FEN \"{fen}\"]"),
        ] {
            assert_board(&format!("{header}\n\n1. e4 *"), 1, "8/8/8/8/4P3/8/8/4K2k b");
        }
    }

    #[test]
    fn black_to_move_fen_takes_blacks_move_first() {
        let pgn = "[FEN \"4k3/4p3/8/8/8/8/8/4K3 b - - 0 1\"]\n\n1... e5 2. Kd2 *";
        assert_board(pgn, 0, "4k3/4p3/8/8/8/8/8/4K3 b");
        assert_board(pgn, 1, "4k3/8/8/4p3/8/8/8/4K3 w");
        assert_board(pgn, 2, "4k3/8/8/4p3/8/8/3K4/8 b");
    }

    #[test]
    fn invalid_fen_tag_is_an_error() {
        for fen in ["not a fen", "8/8/8/8/8/8/8/8 w - - 0 1", "w"] {
            assert_eq!(
                board_at(&format!("[FEN \"{fen}\"]\n1. e4"), 0),
                Err(PgnError::InvalidFen(fen.to_string()))
            );
        }
        // The error carries the unescaped tag value.
        assert_eq!(
            board_at(r#"[FEN "a\"b\\c"]"#, 0),
            Err(PgnError::InvalidFen(r#"a"b\c"#.to_string()))
        );
    }

    #[test]
    fn unsupported_variant_is_an_error() {
        assert_eq!(
            board_at("[Variant \"Crazy\\\"house\"]\n1. e4", 1),
            Err(PgnError::UnsupportedVariant("Crazy\"house".to_string()))
        );
        assert_board("[Variant \"Standard\"]\n1. e4", 0, START);
    }

    #[test]
    fn fen_tag_wins_whatever_setup_says() {
        for setup in ["", "[SetUp \"0\"]\n", "[SetUp \"1\"]\n"] {
            let pgn = format!("{setup}[FEN \"8/8/8/8/8/8/4P3/4K2k w - - 0 1\"]\n1. e4");
            assert_board(&pgn, 0, "8/8/8/8/8/8/4P3/4K2k w");
        }
        // An empty FEN tag is a placeholder for the standard start.
        assert_board("[SetUp \"1\"]\n[FEN \"\"]\n1. e4", 0, START);
    }

    #[test]
    fn lenient_fen_fields_still_render() {
        // A blanket `KQkq` puzzle FEN still renders, and still plays.
        assert_board(
            "[FEN \"4k3/8/8/8/8/8/8/4K2R w KQkq - 0 1\"]\n1. O-O",
            1,
            "4k3/8/8/8/8/8/8/5RK1 b",
        );
        // An en passant square no pawn can capture on is dropped...
        assert_board(
            "[FEN \"4k3/8/8/8/8/8/8/4K3 b - e3 0 1\"]",
            0,
            "4k3/8/8/8/8/8/8/4K3 b",
        );
        // ...but a real one still enables the capture.
        let ep = "[FEN \"4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 1\"]\n1. exd6";
        assert_board(ep, 1, "4k3/8/3P4/8/8/8/8/4K3 b");
        // A malformed en passant field is still invalid.
        let bad = "4k3/8/8/8/8/8/8/4K3 w - e9 0 1";
        assert_eq!(
            board_at(&format!("[FEN \"{bad}\"]"), 0),
            Err(PgnError::InvalidFen(bad.to_string()))
        );
    }

    #[test]
    fn standard_move_variants_play_like_standard() {
        assert_board("[Variant \"Three-check\"]\n1. e4", 0, START);
        assert_board("[Variant \"King of the Hill\"]\n1. e4", 0, START);
        for checks in ["3+3 0 1", "0 1 +0+0"] {
            let pgn = format!("[Variant \"Three-check\"]\n[FEN \"{START} KQkq - {checks}\"]");
            assert_board(&pgn, 0, START);
        }
    }

    #[test]
    fn next_games_fen_tag_is_ignored() {
        let pgn = "1. e4 e5\n\n[FEN \"4k3/8/8/8/8/8/8/4K3 w - - 0 1\"]\n1. Kd2";
        assert_board(pgn, 0, START);
    }

    #[test]
    fn chess960_castling_by_rook_side() {
        let c960 =
            |fen: &str, moves: &str| format!("[Variant \"Chess960\"]\n[FEN \"{fen}\"]\n\n{moves}");
        // King g1 + rook h1: O-O leaves the king in place and moves the rook to f1.
        for rights in ["K", "H"] {
            let pgn = c960(&format!("4k3/8/8/8/8/8/8/6KR w {rights} - 0 1"), "1. O-O");
            assert_board(&pgn, 1, "4k3/8/8/8/8/8/8/5RK1 b");
        }
        // King b1 + rook a1: O-O-O goes to c1/d1.
        for rights in ["Q", "A"] {
            let pgn = c960(&format!("4k3/8/8/8/8/8/8/RK6 w {rights} - 0 1"), "1. O-O-O");
            assert_board(&pgn, 1, "4k3/8/8/8/8/8/8/2KR4 b");
        }
        // King f1 + adjacent rook g1: king and rook swap.
        let pgn = c960("4k3/8/8/8/8/8/8/5KR1 w K - 0 1", "1. O-O");
        assert_board(&pgn, 1, "4k3/8/8/8/8/8/8/5RK1 b");
        // King c1, rooks a1 + b1: `Q` (outermost) is the a-rook, blocked by
        // the b-rook; `B` names the b-rook, and the king doesn't move.
        let blocked = c960("4k3/8/8/8/8/8/8/RRK5 w Q - 0 1", "1. O-O-O");
        assert_eq!(
            board_at(&blocked, 1),
            Err(PgnError::IllegalMove("O-O-O".to_string()))
        );
        let pgn = c960("4k3/8/8/8/8/8/8/RRK5 w B - 0 1", "1. O-O-O");
        assert_board(&pgn, 1, "4k3/8/8/8/8/8/8/R1KR4 b");
        // Black, from a full Chess960 start position (Shredder and X-FEN agree).
        for rights in ["GEge", "KQkq"] {
            let pgn = c960(&format!("{C960_START} w {rights} - 0 1"), "1. O-O O-O");
            assert_board(&pgn, 2, C960_CASTLED);
        }
    }

    #[test]
    fn chess960_without_fen_starts_from_the_standard_position() {
        assert_board(
            "[Variant \"Chess960\"]\n1. e4 e5 2. Nf3 Nc6 3. Bc4 Bc5 4. O-O",
            7,
            "r1bqk1nr/pppp1ppp/2n5/2b1p3/2B1P3/5N2/PPPP1PPP/RNBQ1RK1 b",
        );
    }

    #[test]
    fn chess960_is_inferred_from_an_unambiguous_castling_field() {
        // No Variant tag: Shredder `GEge` is read as Chess960...
        let pgn = format!("[FEN \"{C960_START} w GEge - 0 1\"]\n1. O-O O-O");
        assert_board(&pgn, 2, C960_CASTLED);
        // ...but X-FEN `KQkq` isn't (a sloppy standard puzzle looks the same).
        let pgn = format!("[FEN \"{C960_START} w KQkq - 0 1\"]\n1. O-O");
        assert_eq!(
            board_at(&pgn, 1),
            Err(PgnError::IllegalMove("O-O".to_string()))
        );
        // A standard puzzle: king stepped to f1, blanket `KQkq`, no castling.
        let pgn = "[FEN \"r3k2r/8/8/8/8/8/8/R4K1R w KQkq - 0 1\"]\n1. O-O";
        assert_eq!(
            board_at(pgn, 1),
            Err(PgnError::IllegalMove("O-O".to_string()))
        );
        // An explicit standard Variant tag still wins: the rights are dropped.
        let pgn = format!("[Variant \"Standard\"]\n[FEN \"{C960_START} w GEge - 0 1\"]\n1. O-O");
        assert_eq!(
            board_at(&pgn, 1),
            Err(PgnError::IllegalMove("O-O".to_string()))
        );
        // Standard Shredder rights stay standard and still castle.
        assert_board(
            "[FEN \"r3k2r/8/8/8/8/8/8/R3K2R w HAha - 0 1\"]\n1. O-O",
            1,
            "r3k2r/8/8/8/8/8/8/R4RK1 b",
        );
        // A blanket `KQkq` after standard castling stays standard (`K`/`Q`
        // never infer), so the f-rook `Q` would name in Chess960 can't castle.
        assert_eq!(
            board_at(
                "[FEN \"6k1/5ppp/8/8/8/8/5PPP/5RK1 w KQkq - 0 1\"]\n1. O-O-O",
                1
            ),
            Err(PgnError::IllegalMove("O-O-O".to_string()))
        );
    }

    #[test]
    fn variant_tag_spellings() {
        for absent in [None, Some(""), Some(" ")] {
            assert_eq!(parse_variant(absent), Ok(None));
        }
        for standard in [
            Some("Standard"),
            Some("from position"),
            Some("CHESS"),
            Some("Three-check"),
            Some("king of the hill"),
            Some("King-of-the-Hill"),
        ] {
            assert_eq!(parse_variant(standard), Ok(Some(Variant::Standard)));
        }
        for c960 in ["Chess960", "chess 960", "FischeRandom", "Fischer Random"] {
            assert_eq!(parse_variant(Some(c960)), Ok(Some(Variant::Chess960)));
        }
        assert_eq!(
            parse_variant(Some("Crazyhouse")),
            Err(PgnError::UnsupportedVariant("Crazyhouse".to_string()))
        );
    }

    #[test]
    fn fen_field_shapes() {
        let ok = |fen| parse_fen(fen, Some(Variant::Standard)).is_some();
        assert!(ok("4k3/8/8/8/8/8/8/4K3 w - -"));
        assert!(ok("4k3/8/8/8/8/8/8/4K3  w  -  -  150  80 "));
        assert!(!ok("4k3/8/8/8/8/8/8/4K3 w - - x 1"));
        assert!(!ok("4k3/8/8/8/8/8/8/4K3 w - - 0 1 extra"));
        assert!(!ok("4k3/4K3 w - - 0 1"));
        assert!(!ok("4k3/8/8/8/8/8/8/4K3 w"));
        assert!(!ok(""));
    }
}
