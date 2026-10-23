//! `cozy_chess` position → this crate's [`Board`] model.

use crate::board::{Board, Color, Piece, Role};

pub(super) fn convert_board(pos: &cozy_chess::Board) -> Board {
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
