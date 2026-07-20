//! File/rank coordinate labels drawn inside the edge squares.
//!
//! Coordinate labels (`Options::coordinates`) are drawn lichess-style,
//! tucked inside the corner of the edge squares that carry them (rank digits
//! in the leftmost column, file letters in the bottom row) rather than in a
//! separate margin — that keeps the viewBox at exactly `8 * SQUARE` with no
//! extra layout math for consumers that embed the SVG at a fixed size.

use std::fmt::Write;

use crate::options::Options;

use super::{grid_position, SQUARE};

/// Draws file letters (`a`-`h`) and rank digits (`1`-`8`) into the corners of
/// the edge squares that carry them: rank digits in the leftmost column
/// (top-left corner), file letters in the bottom row (bottom-right corner).
/// Text color is the *opposite* theme color from the square it sits on, so
/// labels stay readable regardless of which color lands on the edge.
pub(super) fn write_coordinates(out: &mut String, opts: &Options) {
    const INSET: u32 = 4;
    for rank in 0..8u8 {
        for file in 0..8u8 {
            let (col, row) = grid_position(file, rank, opts.orientation);
            let is_dark_square = (file + rank) % 2 == 0;
            let label_color = if is_dark_square {
                &opts.theme.light
            } else {
                &opts.theme.dark
            };
            let (x, y) = (u32::from(col) * SQUARE, u32::from(row) * SQUARE);
            if col == 0 {
                let rank_digit = rank + 1;
                let _ = write!(
                    out,
                    r#"<text x="{}" y="{}" font-family="sans-serif" font-size="6" fill="{label_color}">{rank_digit}</text>"#,
                    x + INSET,
                    y + INSET + 6,
                );
            }
            if row == 7 {
                let file_letter = (b'a' + file) as char;
                let _ = write!(
                    out,
                    r#"<text x="{}" y="{}" font-family="sans-serif" font-size="6" fill="{label_color}" text-anchor="end">{file_letter}</text>"#,
                    x + SQUARE - INSET,
                    y + SQUARE - INSET,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::Color;
    use crate::fen::parse;
    use crate::render::svg::SvgRenderer;
    use crate::render::Renderer;

    const START: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

    #[test]
    fn coordinates_emit_16_labels_by_default() {
        let board = parse(START).expect("start position parses");
        let svg = SvgRenderer.render(&board, &Options::default());
        assert_eq!(svg.matches("<text").count(), 16);
    }

    #[test]
    fn coordinates_false_emits_no_text() {
        let board = parse(START).expect("start position parses");
        let opts = Options {
            coordinates: false,
            ..Options::default()
        };
        let svg = SvgRenderer.render(&board, &opts);
        assert!(!svg.contains("<text"));
    }

    #[test]
    fn flipped_orientation_reverses_coordinate_labels() {
        let board = parse(START).expect("start position parses");
        let opts = Options::default();

        // Top-left corner (col 0, row 0) shows rank 8 at White POV, rank 1
        // flipped; both sit on a light square, so the label uses the dark
        // theme color for contrast.
        let white = SvgRenderer.render(&board, &opts);
        let rank_label_white = format!(
            r#"<text x="4" y="10" font-family="sans-serif" font-size="6" fill="{}">8</text>"#,
            opts.theme.dark
        );
        assert!(white.contains(&rank_label_white));

        let black = SvgRenderer.render(
            &board,
            &Options {
                orientation: Color::Black,
                ..Options::default()
            },
        );
        let rank_label_black = format!(
            r#"<text x="4" y="10" font-family="sans-serif" font-size="6" fill="{}">1</text>"#,
            opts.theme.dark
        );
        assert!(black.contains(&rank_label_black));

        // Bottom-left corner (col 0, row 7) shows file "a" at White POV,
        // file "h" flipped; both sit on a dark square, so the label uses the
        // light theme color.
        let file_label_white = format!(
            r#"<text x="41" y="356" font-family="sans-serif" font-size="6" fill="{}" text-anchor="end">a</text>"#,
            opts.theme.light
        );
        assert!(white.contains(&file_label_white));

        let file_label_black = format!(
            r#"<text x="41" y="356" font-family="sans-serif" font-size="6" fill="{}" text-anchor="end">h</text>"#,
            opts.theme.light
        );
        assert!(black.contains(&file_label_black));
    }
}
