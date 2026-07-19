//! FEN → `Board`. Only the placement field and side to move are consumed;
//! castling/en-passant/clock fields are irrelevant for a static diagram and
//! are ignored when present.

use thiserror::Error;

use crate::board::{Board, Color, Piece, Role};

/// The one error type of the crate — FEN parsing is the only fallible step.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum FenError {
    #[error("empty FEN string")]
    Empty,
    #[error("expected 8 ranks separated by '/', found {0}")]
    RankCount(usize),
    #[error("rank {0} does not describe exactly 8 squares")]
    RankWidth(u8),
    #[error("invalid piece character {0:?}")]
    InvalidPiece(char),
    #[error("invalid side to move {0:?}")]
    InvalidSideToMove(String),
}

/// Parses a FEN string (full six-field form or just the placement field)
/// into a [`Board`]. Never panics on any input.
pub fn parse(fen: &str) -> Result<Board, FenError> {
    let mut fields = fen.split_ascii_whitespace();
    let placement = fields.next().ok_or(FenError::Empty)?;

    let ranks: Vec<&str> = placement.split('/').collect();
    if ranks.len() != 8 {
        return Err(FenError::RankCount(ranks.len()));
    }

    let mut squares = [None; 64];
    for (i, rank_str) in ranks.iter().enumerate() {
        // FEN lists rank 8 first; our board index puts a1 at 0.
        let rank = 7 - i as u8;
        let mut file: u8 = 0;
        for c in rank_str.chars() {
            match c {
                '1'..='8' => file += c as u8 - b'0',
                _ => {
                    let piece = piece_from_char(c).ok_or(FenError::InvalidPiece(c))?;
                    if file > 7 {
                        return Err(FenError::RankWidth(rank + 1));
                    }
                    squares[usize::from(rank * 8 + file)] = Some(piece);
                    file += 1;
                }
            }
            if file > 8 {
                return Err(FenError::RankWidth(rank + 1));
            }
        }
        if file != 8 {
            return Err(FenError::RankWidth(rank + 1));
        }
    }

    let side_to_move = match fields.next() {
        None | Some("w") => Color::White,
        Some("b") => Color::Black,
        Some(other) => return Err(FenError::InvalidSideToMove(other.to_string())),
    };

    Ok(Board {
        squares,
        side_to_move,
    })
}

fn piece_from_char(c: char) -> Option<Piece> {
    let role = match c.to_ascii_lowercase() {
        'p' => Role::Pawn,
        'n' => Role::Knight,
        'b' => Role::Bishop,
        'r' => Role::Rook,
        'q' => Role::Queen,
        'k' => Role::King,
        _ => return None,
    };
    let color = if c.is_ascii_uppercase() {
        Color::White
    } else {
        Color::Black
    };
    Some(Piece { color, role })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::Square;

    const START: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

    fn at(board: &Board, sq: &str) -> Option<Piece> {
        board.piece_at(Square::from_algebraic(sq).expect("valid square"))
    }

    #[test]
    fn start_position_spot_checks() {
        let board = parse(START).expect("start position parses");
        assert_eq!(
            at(&board, "e1"),
            Some(Piece {
                color: Color::White,
                role: Role::King
            })
        );
        assert_eq!(
            at(&board, "d8"),
            Some(Piece {
                color: Color::Black,
                role: Role::Queen
            })
        );
        assert_eq!(
            at(&board, "a2"),
            Some(Piece {
                color: Color::White,
                role: Role::Pawn
            })
        );
        assert_eq!(at(&board, "e4"), None);
        assert_eq!(board.side_to_move, Color::White);
    }

    #[test]
    fn start_position_piece_count() {
        let board = parse(START).expect("start position parses");
        let count = (0..64)
            .filter_map(|i| Square::new(i % 8, i / 8))
            .filter(|&sq| board.piece_at(sq).is_some())
            .count();
        assert_eq!(count, 32);
    }

    #[test]
    fn placement_field_alone_is_enough() {
        let board = parse("8/8/8/8/8/8/8/4K3").expect("placement-only FEN parses");
        assert_eq!(
            at(&board, "e1"),
            Some(Piece {
                color: Color::White,
                role: Role::King
            })
        );
        assert_eq!(board.side_to_move, Color::White);
    }

    #[test]
    fn black_to_move() {
        let board = parse("8/8/8/8/8/8/8/4K3 b - - 0 1").expect("valid FEN");
        assert_eq!(board.side_to_move, Color::Black);
    }

    #[test]
    fn digit_gaps_place_pieces_correctly() {
        let board = parse("8/8/8/3q4/8/8/8/8").expect("valid FEN");
        assert_eq!(
            at(&board, "d5"),
            Some(Piece {
                color: Color::Black,
                role: Role::Queen
            })
        );
        assert_eq!(at(&board, "c5"), None);
        assert_eq!(at(&board, "e5"), None);
    }

    #[test]
    fn errors() {
        assert_eq!(parse(""), Err(FenError::Empty));
        assert_eq!(parse("   "), Err(FenError::Empty));
        assert_eq!(parse("8/8/8/8"), Err(FenError::RankCount(4)));
        assert_eq!(parse("8/8/8/8/8/8/8/9"), Err(FenError::InvalidPiece('9')));
        assert_eq!(parse("8/8/8/8/8/8/8/7"), Err(FenError::RankWidth(1)));
        assert_eq!(
            parse("8/8/8/8/8/8/8/ppppppppp"),
            Err(FenError::RankWidth(1))
        );
        assert_eq!(parse("8/8/8/8/8/8/8/7x"), Err(FenError::InvalidPiece('x')));
        assert_eq!(
            parse("8/8/8/8/8/8/8/4K3 x"),
            Err(FenError::InvalidSideToMove("x".into()))
        );
    }
}
