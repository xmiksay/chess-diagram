//! A FEN castling field → castling rights on a `cozy_chess` board. Accepts
//! X-FEN (`KQkq`, `K`/`Q` = the outermost rook; a file letter for an inner
//! one) and Shredder-FEN (`HAha`): cozy's own X-FEN mode pins `K`/`Q` to the
//! h/a files, which is wrong in Chess960. A right the board can't support
//! (puzzle generators often write a blanket `KQkq`) is dropped rather than
//! failing the diagram, as python-chess and lichess do.

use cozy_chess::{Board, BoardBuilder, Color, File, Piece, Rank, Square};

use super::start::Variant;

/// `board` (parsed with no castling rights) with `field`'s rights applied;
/// `None` only when the field is malformed.
pub(super) fn with_castling(board: &Board, field: &str, variant: Variant) -> Option<Board> {
    let mut builder = BoardBuilder::from_board(board);
    if field != "-" {
        for c in field.chars() {
            let letter = c.to_ascii_lowercase();
            if !matches!(letter, 'a'..='h' | 'k' | 'q') {
                return None;
            }
            let color = color_of(c);
            let Some((short, file)) = castling_right(board, color, letter, variant) else {
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

/// The variant a castling field implies on its own, for a FEN exported
/// without a `Variant` tag: Chess960 when some right it grants only exists
/// there (a king off the e-file, or a rook off a/h — e.g. Shredder `GEge`),
/// else standard. Rights no variant supports (a blanket `KQkq` with no
/// rooks) imply nothing.
pub(super) fn implied_variant(board: &Board, field: &str) -> Variant {
    let chess960_only = field.chars().any(|c| {
        let right = |variant| castling_right(board, color_of(c), c.to_ascii_lowercase(), variant);
        right(Variant::Chess960).is_some() && right(Variant::Standard).is_none()
    });
    if chess960_only {
        Variant::Chess960
    } else {
        Variant::Standard
    }
}

/// White for an uppercase castling letter, Black otherwise.
fn color_of(letter: char) -> Color {
    if letter.is_ascii_uppercase() {
        Color::White
    } else {
        Color::Black
    }
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

    fn rights(fen: &str, variant: Variant) -> Option<[(Option<File>, Option<File>); 2]> {
        let (placement, field) = fen.split_once(" w ")?;
        let field = field.split_ascii_whitespace().next()?;
        let board = Board::from_fen(&format!("{placement} w - - 0 1"), false).ok()?;
        let board = with_castling(&board, field, variant)?;
        Some(Color::ALL.map(|c| {
            let r = board.castle_rights(c);
            (r.short, r.long)
        }))
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
}
