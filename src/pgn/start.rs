//! The position a PGN mainline starts from: its `FEN` and `Variant` header
//! tags → a `cozy_chess` board. Chess960 and standard chess share cozy-chess's
//! move generator (castling is always "king takes own rook"), so the variant
//! only changes which castling rights a FEN may grant.
//!
//! The FEN is read leniently where that can't change the board: castling
//! rights the position can't support (puzzle generators often write a blanket
//! `KQkq`) are dropped rather than failing the diagram, as python-chess and
//! lichess do.

use cozy_chess::{Board, BoardBuilder, Color, File, Piece, Rank, Square};

use super::PgnError;

/// `Variant` tag spellings played with standard move rules (lichess writes
/// `Standard` and `From Position`; the rest follow python-chess's aliases).
/// Three-check and King of the Hill only change how a game is won, never
/// which moves are legal, so their boards are walked like standard chess.
const STANDARD: &[&str] = &[
    "Standard",
    "Chess",
    "Normal",
    "Classical",
    "From Position",
    "Three-check",
    "Three check",
    "Threecheck",
    "3check",
    "King of the Hill",
    "KingOfTheHill",
    "KOTH",
];
const CHESS960: &[&str] = &[
    "Chess960",
    "Chess 960",
    "Fischerandom",
    "Fischerrandom",
    "Fischer Random",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Variant {
    Standard,
    Chess960,
}

/// The board a mainline starts from: `fen` (the unescaped `FEN` tag) when
/// present, else the standard start position (also Chess960 position 518).
pub(super) fn start_position(fen: Option<&str>, variant: Option<&str>) -> Result<Board, PgnError> {
    let variant = parse_variant(variant)?;
    match fen {
        None => Ok(Board::startpos()),
        Some(fen) => parse_fen(fen, variant).ok_or_else(|| PgnError::InvalidFen(fen.to_string())),
    }
}

fn parse_variant(tag: Option<&str>) -> Result<Variant, PgnError> {
    // Some exporters write an empty `Variant` tag; treat it as absent.
    let Some(tag) = tag.filter(|t| !t.trim().is_empty()) else {
        return Ok(Variant::Standard);
    };
    let is_one_of = |names: &[&str]| names.iter().any(|n| n.eq_ignore_ascii_case(tag.trim()));
    if is_one_of(STANDARD) {
        Ok(Variant::Standard)
    } else if is_one_of(CHESS960) {
        Ok(Variant::Chess960)
    } else {
        Err(PgnError::UnsupportedVariant(tag.to_string()))
    }
}

/// Parses a FEN whose castling field may be X-FEN (`KQkq`, plus a rook's file
/// letter when it isn't the outermost one) or Shredder-FEN (`HAha`): cozy's
/// own X-FEN mode pins `K`/`Q` to the h/a files, which is wrong in Chess960.
fn parse_fen(fen: &str, variant: Variant) -> Option<Board> {
    let mut fields = fen.split_ascii_whitespace();
    let (placement, side, castling, en_passant) = (
        fields.next()?,
        fields.next()?,
        fields.next()?,
        fields.next()?,
    );
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
    let board = Board::from_fen(&format!("{placement} {side} - {en_passant} 0 1"), false).ok()?;

    let mut builder = BoardBuilder::from_board(&board);
    if castling != "-" {
        for c in castling.chars() {
            let letter = c.to_ascii_lowercase();
            if !matches!(letter, 'a'..='h' | 'k' | 'q') {
                return None;
            }
            let color = if c.is_ascii_uppercase() {
                Color::White
            } else {
                Color::Black
            };
            let Some((short, file)) = castling_right(&board, color, letter, variant) else {
                continue;
            };
            let rights = builder.castle_rights_mut(color);
            let slot = if short {
                &mut rights.short
            } else {
                &mut rights.long
            };
            // A repeated side (`KH`) keeps its first rook.
            slot.get_or_insert(file);
        }
    }
    builder.build().ok()
}

/// One castling-field letter (`k`, `q`, or a rook's file) → (is it the king
/// side, the castling rook's file); `None` when the board can't support that
/// right: no king on the back rank, no rook there, or — in standard chess —
/// anything but an e-file king with an a/h-file rook.
fn castling_right(
    board: &Board,
    color: Color,
    letter: char,
    variant: Variant,
) -> Option<(bool, File)> {
    let back_rank = Rank::First.relative_to(color);
    let king = board.king(color);
    if king.rank() != back_rank {
        return None;
    }
    let king = king.file();
    let mut rook_files = (board.colored_pieces(color, Piece::Rook) & back_rank.bitboard())
        .into_iter()
        .map(Square::file);
    let (short, file) = match letter {
        'k' => (true, rook_files.filter(|&f| f > king).max()?),
        'q' => (false, rook_files.filter(|&f| f < king).min()?),
        _ => {
            let file = File::try_from(letter).ok()?;
            (file > king, rook_files.find(|&f| f == file)?)
        }
    };
    let standard = king == File::E && file == if short { File::H } else { File::A };
    (variant == Variant::Chess960 || standard).then_some((short, file))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pgn::board_at;

    const START: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w";

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
        for fen in ["not a fen", "8/8/8/8/8/8/8/8 w - - 0 1", ""] {
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
    fn setup_zero_overrides_a_fen_tag() {
        let fen = "[FEN \"8/8/8/8/8/8/4P3/4K2k w - - 0 1\"]";
        assert_board(&format!("[SetUp \"0\"]\n{fen}\n1. e4"), 0, START);
        let pgn = format!("[SetUp \"1\"]\n{fen}\n1. e4");
        assert_board(&pgn, 0, "8/8/8/8/8/8/4P3/4K2k w");
    }

    #[test]
    fn standard_move_variants_play_like_standard() {
        assert_board("[Variant \"Three-check\"]\n1. e4", 0, START);
        assert_board("[Variant \"King of the Hill\"]\n1. e4", 0, START);
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
        let start = "bqnbrkrn/pppppppp/8/8/8/8/PPPPPPPP/BQNBRKRN";
        for rights in ["GEge", "KQkq"] {
            let pgn = c960(&format!("{start} w {rights} - 0 1"), "1. O-O O-O");
            assert_board(&pgn, 2, "bqnbrrkn/pppppppp/8/8/8/8/PPPPPPPP/BQNBRRKN w");
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

    fn rights(fen: &str, variant: Variant) -> Option<[(Option<File>, Option<File>); 2]> {
        let board = parse_fen(fen, variant)?;
        Some(Color::ALL.map(|c| {
            let r = board.castle_rights(c);
            (r.short, r.long)
        }))
    }

    #[test]
    fn variant_tag_spellings() {
        for standard in [
            None,
            Some(""),
            Some(" "),
            Some("Standard"),
            Some("from position"),
            Some("CHESS"),
            Some("Three-check"),
            Some("king of the hill"),
        ] {
            assert_eq!(parse_variant(standard), Ok(Variant::Standard));
        }
        for c960 in ["Chess960", "chess 960", "FischeRandom", "Fischer Random"] {
            assert_eq!(parse_variant(Some(c960)), Ok(Variant::Chess960));
        }
        assert_eq!(
            parse_variant(Some("Crazyhouse")),
            Err(PgnError::UnsupportedVariant("Crazyhouse".to_string()))
        );
    }

    #[test]
    fn xfen_letters_pick_the_outermost_rook() {
        // King b1 between rooks a1 and g1/h1; `K` is the h-rook, `G` the g-rook.
        let fen = "r4k1r/8/8/8/8/8/8/RK4RR w KQk - 0 1";
        let h_and_a = Some([(Some(File::H), Some(File::A)), (Some(File::H), None)]);
        assert_eq!(rights(fen, Variant::Chess960), h_and_a);
        assert_eq!(
            rights("r4k1r/8/8/8/8/8/8/RK4RR w HAh - 0 1", Variant::Chess960),
            h_and_a
        );
        assert_eq!(
            rights("r4k1r/8/8/8/8/8/8/RK4RR w GAh - 0 1", Variant::Chess960),
            Some([(Some(File::G), Some(File::A)), (Some(File::H), None)])
        );
    }

    #[test]
    fn unsupportable_castling_rights_are_dropped() {
        let none = Some([(None, None), (None, None)]);
        for fen in [
            "4k3/8/8/8/8/8/8/4K3 w KQkq - 0 1", // blanket rights, no rooks
            "4k3/8/8/8/8/8/8/R3K2R w E - 0 1",  // the king's own file
            "4k3/8/8/8/8/8/8/R3K2R w C - 0 1",  // no rook on that file
            "4k3/8/8/8/8/8/4K3/R6R w KQ - 0 1", // king off the back rank
        ] {
            assert_eq!(rights(fen, Variant::Chess960), none, "{fen}");
        }
        // A repeated side keeps its first rook; the other rights survive.
        assert_eq!(
            rights("4k3/8/8/8/8/8/8/R3K1RR w GHQ - 0 1", Variant::Chess960),
            Some([(Some(File::G), Some(File::A)), (None, None)])
        );
        // A blanket `KQkq` puzzle FEN still renders, and still plays.
        assert_board(
            "[FEN \"4k3/8/8/8/8/8/8/4K2R w KQkq - 0 1\"]\n1. O-O",
            1,
            "4k3/8/8/8/8/8/8/5RK1 b",
        );
    }

    #[test]
    fn malformed_castling_field_is_invalid() {
        for fen in [
            "4k3/8/8/8/8/8/8/R3K2R w X - 0 1",
            "4k3/8/8/8/8/8/8/R3K2R w K1 - 0 1",
        ] {
            assert_eq!(rights(fen, Variant::Chess960), None, "{fen}");
        }
    }

    #[test]
    fn standard_variant_only_castles_from_the_standard_squares() {
        let fen960 = "r4k1r/8/8/8/8/8/8/RK4RR w KQk - 0 1";
        assert_eq!(
            rights(fen960, Variant::Standard),
            Some([(None, None), (None, None)])
        );
        assert_eq!(
            rights("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1", Variant::Standard),
            Some([
                (Some(File::H), Some(File::A)),
                (Some(File::H), Some(File::A))
            ])
        );
    }

    #[test]
    fn fen_field_shapes() {
        let ok = |fen| parse_fen(fen, Variant::Standard).is_some();
        assert!(ok("4k3/8/8/8/8/8/8/4K3 w - -"));
        assert!(ok("4k3/8/8/8/8/8/8/4K3  w  -  -  150  80 "));
        assert!(!ok("4k3/8/8/8/8/8/8/4K3 w - - x 1"));
        assert!(!ok("4k3/8/8/8/8/8/8/4K3 w - - 0 1 extra"));
        assert!(!ok("4k3/4K3 w - - 0 1"));
        assert!(!ok("4k3/8/8/8/8/8/8/4K3 w"));
        assert!(!ok(""));
    }
}
