//! PGN → `Board` at a given ply. `#[cfg(feature = "pgn")]` only: walks the
//! mainline of a PGN game with `shakmaty` (SAN disambiguation, castling, en
//! passant, promotion) and converts the resulting position into this
//! crate's [`Board`]. Never touches the default (`FenError`-only) parse
//! layer contract — errors here are [`PgnError`], a separate type.

use shakmaty::{
    san::San, Chess, Color as ShakmatyColor, Piece as ShakmatyPiece, Position,
    Square as ShakmatySquare,
};
use thiserror::Error;

use crate::board::{Board, Color, Piece, Role};

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
/// Only the mainline is walked: comments (`{...}`), variations (`(...)`),
/// NAGs (`$1`), move numbers, and headers are skipped; a game-result token
/// (`1-0`, `0-1`, `1/2-1/2`, `*`) ends the mainline.
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
    let mut pos = Chess::default();
    for (played, token) in mainline_sans(pgn).enumerate() {
        if played == ply {
            break;
        }
        let san = San::from_ascii(token.as_bytes())
            .map_err(|_| PgnError::InvalidSan(token.to_string()))?;
        let mv = san
            .to_move(&pos)
            .map_err(|_| PgnError::IllegalMove(token.to_string()))?;
        pos = pos
            .play(mv)
            .map_err(|_| PgnError::IllegalMove(token.to_string()))?;
    }

    let available = mainline_sans(pgn).count();
    if ply > available {
        return Err(PgnError::PlyOutOfRange {
            requested: ply,
            available,
        });
    }

    Ok(convert_board(&pos))
}

fn convert_board(pos: &Chess) -> Board {
    let mut squares = [None; 64];
    for i in 0..64u8 {
        let sq = ShakmatySquare::new(u32::from(i));
        if let Some(piece) = pos.board().piece_at(sq) {
            squares[usize::from(i)] = Some(convert_piece(piece));
        }
    }
    Board {
        squares,
        side_to_move: convert_color(pos.turn()),
    }
}

fn convert_piece(piece: ShakmatyPiece) -> Piece {
    Piece {
        color: convert_color(piece.color),
        role: convert_role(piece.role),
    }
}

fn convert_color(color: ShakmatyColor) -> Color {
    match color {
        ShakmatyColor::White => Color::White,
        ShakmatyColor::Black => Color::Black,
    }
}

fn convert_role(role: shakmaty::Role) -> Role {
    match role {
        shakmaty::Role::Pawn => Role::Pawn,
        shakmaty::Role::Knight => Role::Knight,
        shakmaty::Role::Bishop => Role::Bishop,
        shakmaty::Role::Rook => Role::Rook,
        shakmaty::Role::Queen => Role::Queen,
        shakmaty::Role::King => Role::King,
    }
}

/// Yields SAN move tokens from a PGN's mainline, skipping header tags,
/// move numbers, comments, variations, NAGs, and the trailing result token.
///
/// Header tag lines (`[Event "..."]`) are dropped whole, since a quoted tag
/// value may itself contain whitespace; everything else is tokenized
/// word-by-word with brace/paren depth tracked across words so a comment or
/// variation spanning several words is skipped in full.
fn mainline_sans(pgn: &str) -> impl Iterator<Item = &str> {
    let mut brace_depth: u32 = 0;
    let mut paren_depth: u32 = 0;
    pgn.lines()
        .filter(|line| !line.trim_start().starts_with('['))
        .flat_map(str::split_whitespace)
        .filter_map(move |word| {
            if brace_depth > 0 || word.starts_with('{') {
                brace_depth = brace_depth
                    .saturating_add(word.matches('{').count() as u32)
                    .saturating_sub(word.matches('}').count() as u32);
                return None;
            }
            if paren_depth > 0 || word.starts_with('(') {
                paren_depth = paren_depth
                    .saturating_add(word.matches('(').count() as u32)
                    .saturating_sub(word.matches(')').count() as u32);
                return None;
            }
            if word.starts_with('$') {
                return None;
            }
            if matches!(word, "1-0" | "0-1" | "1/2-1/2" | "*") {
                return None;
            }
            let san = word.trim_start_matches(|c: char| c.is_ascii_digit() || c == '.');
            if san.is_empty() {
                return None;
            }
            Some(san)
        })
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
        let available = mainline_sans("1. e4 e5 2. Qh5 Nc6 3. Bc4 Nf6 4. Qxf7# 1-0").count();
        assert_eq!(available, 7);
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
        // Nc3 isn't legal here — no knight can reach c3 from the start position via this SAN? Use a
        // syntactically valid but illegal SAN instead: Ke2 with the king boxed in at move 1.
        assert_eq!(
            board_at("1. Ke2", 1),
            Err(PgnError::IllegalMove("Ke2".to_string()))
        );
    }
}
