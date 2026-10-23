//! PGN → `Board` at a given ply. `#[cfg(feature = "pgn")]` only: walks the
//! mainline of a PGN game, resolving each SAN token (disambiguation,
//! castling, en passant, promotion) against `cozy-chess`'s legal move
//! generator, and converts the resulting position into this crate's
//! [`Board`]. Never touches the default (`FenError`-only) parse layer
//! contract — errors here are [`PgnError`], a separate type.

mod castling;
mod convert;
mod error;
mod movetext;
mod san;
mod start;

use crate::board::Board;
use convert::convert_board;
pub use error::PgnError;
use movetext::{tokens, unescape, Token};
use san::San;
use start::start_position;

/// Walks `pgn`'s mainline to half-move `ply` (`0` = the starting position,
/// `1` = after the first move, …) and returns the resulting [`Board`].
///
/// The walk starts from the position in the game's `FEN` header tag when
/// there is one, whatever `SetUp` says (many exporters omit it), otherwise —
/// or when the tag is empty — from the standard start position. `ply` counts
/// from that position, and the side to move follows the FEN's, so a FEN with
/// Black to move takes Black's move (`1... e5`) as ply 1. Only the header
/// section before the first move is read. The `FEN` and `Variant` tag names
/// match regardless of case (`[Fen ...]`), and a repeated tag's last value
/// wins.
///
/// The `Variant` tag selects the rules: standard chess (`Standard`,
/// `From Position`, …) or a variant with standard move rules
/// (`Three-check`, `King of the Hill`) plays standard chess, as does an
/// absent or empty tag (unless the FEN implies Chess960, below); Chess960
/// (`Chess960`, `Chess 960`, `Fischerandom`, …) accepts X-FEN (`KQkq`,
/// outermost rook) or Shredder-FEN (`HAha`) castling rights and resolves
/// `O-O`/`O-O-O` by rook side. Names compare ignoring case, whitespace, `-`
/// and `_`. Without a `FEN` tag Chess960 starts from the standard position
/// (Chess960 position 518). Without a (non-empty) `Variant` tag, a FEN with
/// a Shredder-FEN castling letter `b`–`g` naming a rook (`GEge`) is read as
/// Chess960;
/// `KQkq`/`HAha` never are, since a sloppy standard puzzle FEN looks the
/// same — add `[Variant "Chess960"]` for such games.
///
/// FEN fields that can't change the board are read leniently: castling rights
/// the position can't support (a blanket `KQkq` with no rooks, a non-standard
/// king square outside Chess960) and an en passant square no pawn could
/// capture on are dropped, not an error.
///
/// Only the mainline is walked: comments (`{...}`, `;`), `%` escape lines,
/// variations (`(...)`), NAGs (`$1`), `!`/`?` suffix annotations, and move
/// numbers are skipped; a game-result token (`1-0`, `0-1`, `1/2-1/2`, `*`),
/// or a tag pair after the first move or after the blank line that closes
/// the header section (the next game, even if this one has no moves), ends
/// the mainline. Castling is accepted as `O-O`/`O-O-O` or zero-castling
/// `0-0`/`0-0-0`.
///
/// # Errors
///
/// [`PgnError::UnsupportedVariant`] for any other `Variant`;
/// [`PgnError::InvalidFen`] when the `FEN` tag doesn't parse;
/// [`PgnError::InvalidSan`] /
/// [`PgnError::IllegalMove`] for a bad mainline move; and
/// [`PgnError::PlyOutOfRange`] when the mainline is shorter than `ply`.
///
/// # Example
///
/// ```
/// use chess_diagram::pgn::board_at;
/// use chess_diagram::Square;
///
/// let board = board_at("1. e4 e5 2. Nf3", 3)?;
/// assert!(board.piece_at(Square::from_algebraic("f3").unwrap()).is_some());
///
/// // A `FEN` tag sets the starting position.
/// let endgame = "[FEN \"8/8/8/8/8/8/4P3/4K2k w - - 0 1\"]\n\n1. e4 *";
/// let board = board_at(endgame, 1)?;
/// assert!(board.piece_at(Square::from_algebraic("d1").unwrap()).is_none());
/// # Ok::<(), chess_diagram::pgn::PgnError>(())
/// ```
pub fn board_at(pgn: &str, ply: usize) -> Result<Board, PgnError> {
    let mut tokens = tokens(pgn).peekable();
    let (mut fen, mut variant) = (None, None);
    while let Some(Token::Tag(tag)) = tokens.next_if(|t| matches!(t, Token::Tag(_))) {
        // The spec makes tag names case-sensitive, but a mis-cased `[Fen]`
        // silently ignored would render the wrong board.
        if tag.name.eq_ignore_ascii_case("FEN") {
            fen = Some(unescape(tag.value));
        } else if tag.name.eq_ignore_ascii_case("Variant") {
            variant = Some(unescape(tag.value));
        }
    }
    let mut pos = start_position(fen.as_deref(), variant.as_deref())?;

    let mut played = 0;
    // Only SAN tokens follow the header: a later tag pair ends the lexer.
    for token in tokens.filter_map(Token::san).take(ply) {
        let san = San::parse(token).ok_or_else(|| PgnError::InvalidSan(token.to_string()))?;
        let mv = san
            .to_move(&pos)
            .ok_or_else(|| PgnError::IllegalMove(token.to_string()))?;
        // `mv` came from the legal move list, so this can't fail today; kept
        // over `play_unchecked` because that one panics on an illegal move.
        pos.try_play(mv)
            .map_err(|_| PgnError::IllegalMove(token.to_string()))?;
        played += 1;
    }

    // `take(ply)` only stops short when the mainline runs out.
    if played < ply {
        return Err(PgnError::PlyOutOfRange {
            requested: ply,
            available: played,
        });
    }

    Ok(convert_board(&pos))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::{Color, Piece, Role, Square};

    #[test]
    fn fen_and_variant_tag_names_ignore_case() {
        let expected = crate::fen::parse("8/8/8/8/8/8/4P3/4K2k w").expect("valid FEN");
        let fen = "8/8/8/8/8/8/4P3/4K2k w - - 0 1";
        // A repeat differing only in case still counts as the same tag: last wins.
        for header in [
            "[Fen \"{f}\"]",
            "[fen \"{f}\"]",
            "[FEN \"\"]\n[fEN \"{f}\"]",
        ] {
            let pgn = format!("{}\n1. e4", header.replace("{f}", fen));
            assert_eq!(board_at(&pgn, 0), Ok(expected.clone()), "{pgn}");
        }
        let c960 = "[variant \"Chess960\"]\n[FEN \"4k3/8/8/8/8/8/8/5KR1 w K - 0 1\"]\n1. O-O";
        let castled = crate::fen::parse("4k3/8/8/8/8/8/8/5RK1 b").expect("valid FEN");
        assert_eq!(board_at(c960, 1), Ok(castled));
    }

    #[test]
    fn ply_zero_is_start_position() {
        let board = board_at("1. e4 e5", 0).expect("valid PGN");
        assert_eq!(
            board.piece_at(Square::from_algebraic("e1").unwrap()),
            Some(Piece {
                color: Color::White,
                role: Role::King
            })
        );
        assert_eq!(board.side_to_move, Color::White);
    }

    #[test]
    fn mainline_walk_advances_side_to_move() {
        let board = board_at("1. e4 e5 2. Nf3", 3).expect("valid PGN");
        assert_eq!(
            board.piece_at(Square::from_algebraic("f3").unwrap()),
            Some(Piece {
                color: Color::White,
                role: Role::Knight
            })
        );
        assert_eq!(board.side_to_move, Color::Black);
    }

    #[test]
    fn zero_castling_matches_letter_castling() {
        let moves = "1. e4 e5 2. Nf3 Nc6 3. Bc4 Bc5 4.";
        let zero = board_at(&format!("{moves} 0-0"), 7);
        assert!(zero.is_ok());
        assert_eq!(zero, board_at(&format!("{moves} O-O"), 7));
        // Black castling queenside, glued to its move number.
        let moves = "1. d4 d5 2. Nc3 Nc6 3. Bf4 Bf5 4. Qd2 Qd7 5. e3";
        let zero = board_at(&format!("{moves} 5...0-0-0"), 10);
        assert!(zero.is_ok());
        assert_eq!(zero, board_at(&format!("{moves} 5...O-O-O"), 10));
    }

    #[test]
    fn castling_kingside() {
        // 1.e4 e5 2.Nf3 Nc6 3.Bc4 Bc5 4.O-O -- White castles kingside.
        let board = board_at("1. e4 e5 2. Nf3 Nc6 3. Bc4 Bc5 4. O-O", 7).expect("valid PGN");
        assert_eq!(
            board.piece_at(Square::from_algebraic("g1").unwrap()),
            Some(Piece {
                color: Color::White,
                role: Role::King
            })
        );
        assert_eq!(
            board.piece_at(Square::from_algebraic("f1").unwrap()),
            Some(Piece {
                color: Color::White,
                role: Role::Rook
            })
        );
        assert_eq!(board.piece_at(Square::from_algebraic("e1").unwrap()), None);
        assert_eq!(board.piece_at(Square::from_algebraic("h1").unwrap()), None);
    }

    #[test]
    fn promotion_to_queen() {
        // White walks a pawn b2-b4-b5-b6, captures a7, then captures the
        // b8 knight while promoting.
        let pgn = "1. b4 h6 2. b5 h5 3. b6 h4 4. bxa7 h3 5. axb8=Q";
        let board = board_at(pgn, 9).expect("valid PGN");
        assert_eq!(
            board.piece_at(Square::from_algebraic("b8").unwrap()),
            Some(Piece {
                color: Color::White,
                role: Role::Queen
            })
        );
        assert_eq!(board.piece_at(Square::from_algebraic("a7").unwrap()), None);
    }

    #[test]
    fn en_passant_capture() {
        // 1.e4 a6 2.e5 d5 3.exd6 -- White captures en passant on d6.
        let board = board_at("1. e4 a6 2. e5 d5 3. exd6", 5).expect("valid PGN");
        assert_eq!(
            board.piece_at(Square::from_algebraic("d6").unwrap()),
            Some(Piece {
                color: Color::White,
                role: Role::Pawn
            })
        );
        assert_eq!(board.piece_at(Square::from_algebraic("d5").unwrap()), None);
        assert_eq!(board.piece_at(Square::from_algebraic("e5").unwrap()), None);
    }

    #[test]
    fn comments_and_nags_are_skipped() {
        let pgn = "1. e4 {best by test} e5 2. Nf3 $1 Nc6";
        let board = board_at(pgn, 4).expect("valid PGN");
        assert_eq!(
            board.piece_at(Square::from_algebraic("c6").unwrap()),
            Some(Piece {
                color: Color::Black,
                role: Role::Knight
            })
        );
    }

    #[test]
    fn variations_are_skipped() {
        let pgn = "1. e4 e5 (1... c5 2. Nf3) 2. Nf3 Nc6";
        let board = board_at(pgn, 4).expect("valid PGN");
        assert_eq!(
            board.piece_at(Square::from_algebraic("c6").unwrap()),
            Some(Piece {
                color: Color::Black,
                role: Role::Knight
            })
        );
    }

    #[test]
    fn result_token_ends_mainline() {
        // A second concatenated game after the result is not part of this mainline.
        let two_games = "1. e4 e5 1-0\n\n[Event \"Next\"]\n1. d4 d5 *";
        assert_eq!(
            board_at(two_games, 3),
            Err(PgnError::PlyOutOfRange {
                requested: 3,
                available: 2
            })
        );
    }

    #[test]
    fn next_game_headers_end_a_resultless_mainline() {
        let two_games = "1. e4 e5\n\n[Event \"Next\"]\n1. d4 d5";
        assert_eq!(
            board_at(two_games, 3),
            Err(PgnError::PlyOutOfRange {
                requested: 3,
                available: 2
            })
        );
    }

    #[test]
    fn a_moveless_games_successor_is_not_read_as_it() {
        let pgn = "[Event \"a\"]\n[FEN \"4k3/8/8/8/8/8/8/4K3 w - - 0 1\"]\n\n[Event \"b\"]\n1. e4";
        let fen_board = crate::fen::parse("4k3/8/8/8/8/8/8/4K3 w").expect("valid FEN");
        assert_eq!(board_at(pgn, 0), Ok(fen_board));
        assert_eq!(
            board_at(pgn, 1),
            Err(PgnError::PlyOutOfRange {
                requested: 1,
                available: 0
            })
        );
    }

    #[test]
    fn multi_word_header_values_do_not_leak_into_mainline() {
        let pgn =
            "[Event \"World Championship\"]\n[White \"Kasparov, Garry\"]\n\n1. e4 e5 2. Nf3 Nc6";
        let board = board_at(pgn, 4).expect("valid PGN");
        assert_eq!(
            board.piece_at(Square::from_algebraic("c6").unwrap()),
            Some(Piece {
                color: Color::Black,
                role: Role::Knight
            })
        );
    }

    #[test]
    fn opera_game_to_mate() {
        // Morphy's Opera Game: disambiguation (Nbd7), queenside castling,
        // check/mate suffixes, and the result token.
        let pgn = "1. e4 e5 2. Nf3 d6 3. d4 Bg4 4. dxe5 Bxf3 5. Qxf3 dxe5 6. Bc4 Nf6 \
                   7. Qb3 Qe7 8. Nc3 c6 9. Bg5 b5 10. Nxb5 cxb5 11. Bxb5+ Nbd7 \
                   12. O-O-O Rd8 13. Rxd7 Rxd7 14. Rd1 Qe6 15. Bxd7+ Nxd7 \
                   16. Qb8+ Nxb8 17. Rd8# 1-0";
        let board = board_at(pgn, 33).expect("valid PGN");
        let at = |sq| board.piece_at(Square::from_algebraic(sq).unwrap());
        let white = |role| {
            Some(Piece {
                color: Color::White,
                role,
            })
        };
        assert_eq!(at("d8"), white(Role::Rook));
        assert_eq!(at("c1"), white(Role::King));
        assert_eq!(at("g5"), white(Role::Bishop));
        assert_eq!(
            at("b8"),
            Some(Piece {
                color: Color::Black,
                role: Role::Knight
            })
        );
        assert_eq!(board.side_to_move, Color::Black);
    }

    #[test]
    fn ambiguous_san_is_illegal() {
        // Both the b1 and f3 knights can reach the vacated d2.
        assert_eq!(
            board_at("1. d4 a6 2. Nf3 a5 3. Nd2", 5),
            Err(PgnError::IllegalMove("Nd2".to_string()))
        );
        assert!(board_at("1. d4 a6 2. Nf3 a5 3. Nbd2", 5).is_ok());
    }

    #[test]
    fn annotated_moves_play() {
        let board = board_at("1. e4! e5?! 2. Nf3!! Nc6", 4).expect("valid PGN");
        assert_eq!(board.side_to_move, Color::White);
    }

    #[test]
    fn ply_out_of_range() {
        assert_eq!(
            board_at("1. e4 e5", 5),
            Err(PgnError::PlyOutOfRange {
                requested: 5,
                available: 2
            })
        );
    }

    #[test]
    fn invalid_san_token() {
        assert_eq!(
            board_at("1. e4 xyz", 2),
            Err(PgnError::InvalidSan("xyz".to_string()))
        );
    }

    #[test]
    fn illegal_move() {
        // Syntactically valid but illegal: the king is boxed in at move 1.
        assert_eq!(
            board_at("1. Ke2", 1),
            Err(PgnError::IllegalMove("Ke2".to_string()))
        );
    }
}
