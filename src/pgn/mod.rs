//! PGN → `Board` at a given ply. `#[cfg(feature = "pgn")]` only: walks the
//! mainline of a PGN game, resolving each SAN token (disambiguation,
//! castling, en passant, promotion) against `cozy-chess`'s legal move
//! generator, and converts the resulting position into this crate's
//! [`Board`]. Never touches the default (`FenError`-only) parse layer
//! contract — errors here are [`PgnError`], a separate type.

mod movetext;
mod san;

use thiserror::Error;

use crate::board::{Board, Color, Piece, Role};
use movetext::mainline_sans;
use san::San;

/// Errors from walking a PGN movetext. Kept separate from [`FenError`](crate::FenError)
/// so the default build's error surface is unaffected by this feature.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum PgnError {
    /// `ply` asked for more half-moves than the mainline contains.
    #[error("PGN mainline has {available} ply, requested ply {requested}")]
    PlyOutOfRange {
        /// The ply that was requested.
        requested: usize,
        /// The number of mainline ply actually present.
        available: usize,
    },
    /// A SAN token in the mainline didn't parse.
    #[error("invalid SAN token {0:?}")]
    InvalidSan(String),
    /// A SAN token parsed but doesn't name a legal move in its position.
    #[error("illegal move {0:?}")]
    IllegalMove(String),
}

/// Walks `pgn`'s mainline to half-move `ply` (`0` = the starting position,
/// `1` = after White's first move, …) and returns the resulting [`Board`].
///
/// Only the mainline is walked: comments (`{...}`, `;`), `%` escape lines,
/// variations (`(...)`), NAGs (`$1`), `!`/`?` suffix annotations, move
/// numbers, and tag pairs are skipped; a game-result token (`1-0`, `0-1`,
/// `1/2-1/2`, `*`) ends the mainline.
///
/// # Example
///
/// ```
/// use chess_diagram::pgn::board_at;
///
/// let board = board_at("1. e4 e5 2. Nf3", 3)?;
/// assert!(board.piece_at(chess_diagram::Square::from_algebraic("f3").unwrap()).is_some());
/// # Ok::<(), chess_diagram::pgn::PgnError>(())
/// ```
pub fn board_at(pgn: &str, ply: usize) -> Result<Board, PgnError> {
    let mut pos = cozy_chess::Board::startpos();
    let mut played = 0;
    for token in mainline_sans(pgn).take(ply) {
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

fn convert_board(pos: &cozy_chess::Board) -> Board {
    let mut squares = [None; 64];
    // Both models index squares rank-major from a1, so the orders line up.
    for (slot, sq) in squares.iter_mut().zip(cozy_chess::Square::ALL) {
        *slot = pos
            .piece_on(sq)
            .zip(pos.color_on(sq))
            .map(|(role, color)| Piece {
                color: convert_color(color),
                role: convert_role(role),
            });
    }
    Board {
        squares,
        side_to_move: convert_color(pos.side_to_move()),
    }
}

fn convert_color(color: cozy_chess::Color) -> Color {
    match color {
        cozy_chess::Color::White => Color::White,
        cozy_chess::Color::Black => Color::Black,
    }
}

fn convert_role(role: cozy_chess::Piece) -> Role {
    match role {
        cozy_chess::Piece::Pawn => Role::Pawn,
        cozy_chess::Piece::Knight => Role::Knight,
        cozy_chess::Piece::Bishop => Role::Bishop,
        cozy_chess::Piece::Rook => Role::Rook,
        cozy_chess::Piece::Queen => Role::Queen,
        cozy_chess::Piece::King => Role::King,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::Square;

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
