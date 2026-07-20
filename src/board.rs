//! Position model — the parsed form every renderer consumes.
//!
//! No format name and no SVG string may ever appear in this layer; rendering
//! concerns live behind the `Renderer` trait.

/// Piece color; doubles as board orientation (point of view) in `Options`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Color {
    /// White, the default orientation and side to move.
    #[default]
    White,
    /// Black.
    Black,
}

/// Piece kind, independent of color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// Pawn.
    Pawn,
    /// Knight.
    Knight,
    /// Bishop.
    Bishop,
    /// Rook.
    Rook,
    /// Queen.
    Queen,
    /// King.
    King,
}

/// A colored piece occupying a square.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Piece {
    /// The piece's color.
    pub color: Color,
    /// The piece's kind.
    pub role: Role,
}

/// Board square index `0..64`, `a1 = 0`, `b1 = 1`, …, `h8 = 63`.
///
/// # Example
///
/// ```
/// use chess_diagram::Square;
///
/// let e4 = Square::from_algebraic("e4").unwrap();
/// assert_eq!(e4.file(), 4);
/// assert_eq!(e4.rank(), 3);
/// assert_eq!(Square::from_algebraic("i9"), None);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Square(u8);

impl Square {
    /// Builds a square from zero-based file (`a = 0`) and rank (`1 = 0`).
    pub fn new(file: u8, rank: u8) -> Option<Square> {
        (file < 8 && rank < 8).then(|| Square(rank * 8 + file))
    }

    /// Parses algebraic notation like `"e4"`; `None` for anything else (wrong
    /// length, out-of-range file/rank). Never panics on any input.
    pub fn from_algebraic(s: &str) -> Option<Square> {
        let bytes = s.as_bytes();
        if bytes.len() != 2 {
            return None;
        }
        let file = bytes[0].checked_sub(b'a')?;
        let rank = bytes[1].checked_sub(b'1')?;
        Square::new(file, rank)
    }

    /// Zero-based file, `a = 0` … `h = 7`.
    pub fn file(self) -> u8 {
        self.0 % 8
    }

    /// Zero-based rank, first rank = 0.
    pub fn rank(self) -> u8 {
        self.0 / 8
    }

    /// Index into a 64-element board array.
    pub fn index(self) -> usize {
        usize::from(self.0)
    }
}

/// A parsed position: piece placement plus side to move.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Board {
    pub(crate) squares: [Option<Piece>; 64],
    /// The side to move, from the FEN's second field (default `White` if
    /// absent).
    pub side_to_move: Color,
}

impl Board {
    /// Piece on `square`, if any.
    pub fn piece_at(&self, square: Square) -> Option<Piece> {
        self.squares[square.index()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn square_from_algebraic_corners() {
        assert_eq!(Square::from_algebraic("a1"), Square::new(0, 0));
        assert_eq!(Square::from_algebraic("h8"), Square::new(7, 7));
        assert_eq!(Square::from_algebraic("e4"), Square::new(4, 3));
    }

    #[test]
    fn square_from_algebraic_rejects_garbage() {
        for s in ["", "e", "e44", "i1", "a9", "4e", "  "] {
            assert_eq!(Square::from_algebraic(s), None, "{s:?}");
        }
    }

    #[test]
    fn square_file_rank_index_roundtrip() {
        let sq = Square::from_algebraic("c6").expect("valid square");
        assert_eq!(sq.file(), 2);
        assert_eq!(sq.rank(), 5);
        assert_eq!(sq.index(), 5 * 8 + 2);
    }
}
