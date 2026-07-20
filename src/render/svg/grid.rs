//! Base board squares and highlight/check overlay rects.
//!
//! `Options::highlight`/`Options::check` are drawn as semi-transparent
//! overlay rects between the base squares and the pieces, so a tinted square
//! still shows its piece on top. Colors come from `Theme`; only the overlay
//! opacity is a renderer constant, since it's a rendering detail, not a color
//! choice.

use std::fmt::Write;

use crate::board::Square;
use crate::options::Options;

use super::{grid_position, SQUARE};

/// Opacity of the highlight/check overlay rects, so the tinted square color
/// still shows through from `Theme` underneath.
const OVERLAY_OPACITY: f32 = 0.5;

/// Draws the 64 base squares, alternating `Theme::dark`/`Theme::light` in a
/// checkerboard pattern, orientation-aware via [`grid_position`].
pub(super) fn draw_squares(out: &mut String, opts: &Options) {
    for rank in 0..8u8 {
        for file in 0..8u8 {
            let (col, row) = grid_position(file, rank, opts.orientation);
            let fill = if (file + rank) % 2 == 0 {
                &opts.theme.dark
            } else {
                &opts.theme.light
            };
            let (x, y) = (u32::from(col) * SQUARE, u32::from(row) * SQUARE);
            let _ = write!(
                out,
                r#"<rect x="{x}" y="{y}" width="{SQUARE}" height="{SQUARE}" fill="{fill}"/>"#
            );
        }
    }
}

/// Draws a semi-transparent tinted rect over `square`, orientation-aware via
/// the same `grid_position` helper the grid and pieces use.
pub(super) fn draw_overlay(out: &mut String, square: Square, opts: &Options, fill: &str) {
    let (col, row) = grid_position(square.file(), square.rank(), opts.orientation);
    let (x, y) = (u32::from(col) * SQUARE, u32::from(row) * SQUARE);
    let _ = write!(
        out,
        r#"<rect x="{x}" y="{y}" width="{SQUARE}" height="{SQUARE}" fill="{fill}" fill-opacity="{OVERLAY_OPACITY}"/>"#
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::Color;
    use crate::fen::parse;
    use crate::render::svg::SvgRenderer;
    use crate::render::Renderer;

    const START: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

    fn overlay_rect(x: u32, y: u32, fill: &str) -> String {
        format!(
            r#"<rect x="{x}" y="{y}" width="{SQUARE}" height="{SQUARE}" fill="{fill}" fill-opacity="{OVERLAY_OPACITY}"/>"#
        )
    }

    #[test]
    fn a1_is_dark() {
        let board = parse(START).expect("start position parses");
        let opts = Options::default();
        let svg = SvgRenderer.render(&board, &opts);
        // White POV: a1 sits bottom-left at grid (0, 7) -> pixel (0, 315).
        let a1 = format!(
            r#"<rect x="0" y="315" width="45" height="45" fill="{}"/>"#,
            opts.theme.dark
        );
        assert!(svg.contains(&a1));
    }

    #[test]
    fn highlight_draws_two_overlay_rects_white_pov() {
        let board = parse(START).expect("start position parses");
        let opts = Options {
            highlight: vec![
                Square::from_algebraic("e2").expect("valid square"),
                Square::from_algebraic("e4").expect("valid square"),
            ],
            ..Options::default()
        };
        let svg = SvgRenderer.render(&board, &opts);
        // White POV: e2 -> grid (4, 6) -> (180, 270); e4 -> grid (4, 4) -> (180, 180).
        assert!(svg.contains(&overlay_rect(180, 270, &opts.theme.highlight)));
        assert!(svg.contains(&overlay_rect(180, 180, &opts.theme.highlight)));
        assert_eq!(
            svg.matches(&format!(r#"fill="{}""#, opts.theme.highlight))
                .count(),
            2
        );
    }

    #[test]
    fn highlight_draws_two_overlay_rects_black_pov() {
        let board = parse(START).expect("start position parses");
        let opts = Options {
            orientation: Color::Black,
            highlight: vec![
                Square::from_algebraic("e2").expect("valid square"),
                Square::from_algebraic("e4").expect("valid square"),
            ],
            ..Options::default()
        };
        let svg = SvgRenderer.render(&board, &opts);
        // Black POV: e2 -> grid (3, 1) -> (135, 45); e4 -> grid (3, 3) -> (135, 135).
        assert!(svg.contains(&overlay_rect(135, 45, &opts.theme.highlight)));
        assert!(svg.contains(&overlay_rect(135, 135, &opts.theme.highlight)));
    }

    #[test]
    fn check_draws_tint_at_the_right_square() {
        let board = parse(START).expect("start position parses");
        let opts = Options {
            check: Square::from_algebraic("e8"),
            ..Options::default()
        };
        let svg = SvgRenderer.render(&board, &opts);
        // White POV: e8 -> grid (4, 0) -> (180, 0).
        assert!(svg.contains(&overlay_rect(180, 0, &opts.theme.check)));
    }

    #[test]
    fn check_none_emits_no_check_tint() {
        let board = parse(START).expect("start position parses");
        let opts = Options::default();
        let svg = SvgRenderer.render(&board, &opts);
        assert!(!svg.contains(&opts.theme.check));
    }

    #[test]
    fn highlight_overlay_is_drawn_under_pieces() {
        let board = parse(START).expect("start position parses");
        let opts = Options {
            highlight: vec![Square::from_algebraic("e2").expect("valid square")],
            ..Options::default()
        };
        let svg = SvgRenderer.render(&board, &opts);
        let overlay_pos = svg
            .find(&overlay_rect(180, 270, &opts.theme.highlight))
            .expect("overlay rect present");
        let piece_pos = svg
            .find(r#"<g transform="translate("#)
            .expect("at least one piece present");
        assert!(overlay_pos < piece_pos);
    }
}
