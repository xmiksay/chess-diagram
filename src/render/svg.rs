//! Hand-emitted SVG renderer — strings only, no dependencies.
//!
//! Emits the board grid (orientation- and theme-aware) with the Cburnett
//! pieces (`crate::pieces`) placed on top. The viewBox is `0 0 360 360` — 45
//! SVG units per square, matching the Cburnett glyphs' native 45×45
//! coordinate system exactly — so squares and glyphs both use integral
//! coordinates and pieces need only a `translate`, no `scale`.
//!
//! Coordinate labels (`Options::coordinates`) are drawn lichess-style,
//! tucked inside the corner of the edge squares that carry them (rank digits
//! in the leftmost column, file letters in the bottom row) rather than in a
//! separate margin — that keeps the viewBox at exactly `8 * SQUARE` with no
//! extra layout math for consumers that embed the SVG at a fixed size.
//!
//! `Options::highlight`/`Options::check` are drawn as semi-transparent
//! overlay rects between the base squares and the pieces, so a tinted square
//! still shows its piece on top. Colors come from `Theme`; only the overlay
//! opacity is a renderer constant, since it's a rendering detail, not a color
//! choice.

use std::fmt::Write;

use crate::board::{Board, Color, Square};
use crate::options::{Format, Options};
use crate::pieces;
use crate::render::Renderer;

/// SVG units per square; matches the Cburnett glyphs' native viewBox.
const SQUARE: u32 = 45;

/// Opacity of the highlight/check overlay rects, so the tinted square color
/// still shows through from `Theme` underneath.
const OVERLAY_OPACITY: f32 = 0.5;

/// Maps a (file, rank) pair — both zero-based, rank 0 = rank 1 — to the grid
/// column/row it's drawn at under the given orientation. Column/row `0` is
/// the top-left square; White POV puts rank 8 there, Black POV puts rank 1.
fn grid_position(file: u8, rank: u8, orientation: Color) -> (u8, u8) {
    match orientation {
        Color::White => (file, 7 - rank),
        Color::Black => (7 - file, rank),
    }
}

/// [`Renderer`] impl that emits hand-rolled, self-contained SVG.
pub struct SvgRenderer;

impl Renderer for SvgRenderer {
    fn format(&self) -> Format {
        Format::Svg
    }

    fn render(&self, board: &Board, opts: &Options) -> String {
        let size = opts.size;
        let viewbox = SQUARE * 8;
        let mut out = String::with_capacity(4096);
        let _ = write!(
            out,
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{size}" height="{size}" viewBox="0 0 {viewbox} {viewbox}">"#
        );
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
        for square in &opts.highlight {
            draw_overlay(&mut out, *square, opts, &opts.theme.highlight);
        }
        if let Some(square) = opts.check {
            draw_overlay(&mut out, square, opts, &opts.theme.check);
        }
        for rank in 0..8u8 {
            for file in 0..8u8 {
                let Some(square) = Square::new(file, rank) else {
                    continue;
                };
                let Some(piece) = board.piece_at(square) else {
                    continue;
                };
                let (col, row) = grid_position(file, rank, opts.orientation);
                let (x, y) = (u32::from(col) * SQUARE, u32::from(row) * SQUARE);
                let glyph = pieces::glyph(piece);
                let _ = write!(out, r#"<g transform="translate({x} {y})">{glyph}</g>"#);
            }
        }
        if opts.coordinates {
            write_coordinates(&mut out, opts);
        }
        out.push_str("</svg>");
        out
    }
}

/// Draws a semi-transparent tinted rect over `square`, orientation-aware via
/// the same `grid_position` helper the grid and pieces use.
fn draw_overlay(out: &mut String, square: Square, opts: &Options, fill: &str) {
    let (col, row) = grid_position(square.file(), square.rank(), opts.orientation);
    let (x, y) = (u32::from(col) * SQUARE, u32::from(row) * SQUARE);
    let _ = write!(
        out,
        r#"<rect x="{x}" y="{y}" width="{SQUARE}" height="{SQUARE}" fill="{fill}" fill-opacity="{OVERLAY_OPACITY}"/>"#
    );
}

/// Draws file letters (`a`-`h`) and rank digits (`1`-`8`) into the corners of
/// the edge squares that carry them: rank digits in the leftmost column
/// (top-left corner), file letters in the bottom row (bottom-right corner).
/// Text color is the *opposite* theme color from the square it sits on, so
/// labels stay readable regardless of which color lands on the edge.
fn write_coordinates(out: &mut String, opts: &Options) {
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
    use crate::board::{Piece, Role};
    use crate::fen::parse;

    const START: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

    fn piece_at(svg: &str, x: u32, y: u32, piece: Piece) -> bool {
        let tag = format!(
            r#"<g transform="translate({x} {y})">{}</g>"#,
            pieces::glyph(piece)
        );
        svg.contains(&tag)
    }

    #[test]
    fn reports_svg_format() {
        assert_eq!(SvgRenderer.format(), Format::Svg);
    }

    #[test]
    fn emits_self_contained_svg() {
        let board = parse(START).expect("start position parses");
        let svg = SvgRenderer.render(&board, &Options::default());
        assert!(svg.starts_with("<svg "));
        assert!(svg.ends_with("</svg>"));
        assert!(svg.contains(r#"xmlns="http://www.w3.org/2000/svg""#));
        assert_eq!(svg.matches("<rect").count(), 64);
    }

    #[test]
    fn start_position_places_32_pieces() {
        let board = parse(START).expect("start position parses");
        let svg = SvgRenderer.render(&board, &Options::default());
        assert_eq!(svg.matches(r#"transform="translate("#).count(), 32);
    }

    #[test]
    fn white_pov_places_pieces_on_correct_squares() {
        let board = parse(START).expect("start position parses");
        let svg = SvgRenderer.render(&board, &Options::default());
        // White POV: e1 at grid (4, 7) -> (180, 315); e8 at (4, 0) -> (180, 0);
        // a2 at (0, 6) -> (0, 270).
        assert!(piece_at(
            &svg,
            180,
            315,
            Piece {
                color: Color::White,
                role: Role::King
            }
        ));
        assert!(piece_at(
            &svg,
            180,
            0,
            Piece {
                color: Color::Black,
                role: Role::King
            }
        ));
        assert!(piece_at(
            &svg,
            0,
            270,
            Piece {
                color: Color::White,
                role: Role::Pawn
            }
        ));
    }

    #[test]
    fn orientation_flips_the_grid() {
        let board = parse(START).expect("start position parses");
        let white = SvgRenderer.render(&board, &Options::default());
        let black = SvgRenderer.render(
            &board,
            &Options {
                orientation: Color::Black,
                ..Options::default()
            },
        );
        assert_ne!(white, black);
    }

    #[test]
    fn flipped_orientation_puts_white_pieces_at_the_top() {
        let board = parse(START).expect("start position parses");
        let svg = SvgRenderer.render(
            &board,
            &Options {
                orientation: Color::Black,
                ..Options::default()
            },
        );
        // Black POV: e1 -> grid (3, 7) -> (135, 315); e8 -> (3, 0) -> (135, 0).
        assert!(piece_at(
            &svg,
            135,
            0,
            Piece {
                color: Color::White,
                role: Role::King
            }
        ));
        assert!(piece_at(
            &svg,
            135,
            315,
            Piece {
                color: Color::Black,
                role: Role::King
            }
        ));
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

    fn overlay_rect(x: u32, y: u32, fill: &str) -> String {
        format!(
            r#"<rect x="{x}" y="{y}" width="{SQUARE}" height="{SQUARE}" fill="{fill}" fill-opacity="{OVERLAY_OPACITY}"/>"#
        )
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
