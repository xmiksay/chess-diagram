//! Hand-emitted SVG renderer — strings only, no dependencies.
//!
//! Emits the board grid ([`grid`], orientation- and theme-aware) with the
//! Cburnett pieces (`crate::pieces`) placed on top, then coordinate labels
//! ([`coordinates`]) when enabled. The viewBox is `0 0 360 360` — 45 SVG
//! units per square, matching the Cburnett glyphs' native 45×45 coordinate
//! system exactly — so squares and glyphs both use integral coordinates and
//! pieces need only a `translate`, no `scale`.

use std::fmt::Write;

use crate::board::{Board, Color, Square};
use crate::options::{Format, Options};
use crate::pieces;
use crate::render::Renderer;

mod coordinates;
mod grid;

/// SVG units per square; matches the Cburnett glyphs' native viewBox.
pub(super) const SQUARE: u32 = 45;

/// Maps a (file, rank) pair — both zero-based, rank 0 = rank 1 — to the grid
/// column/row it's drawn at under the given orientation. Column/row `0` is
/// the top-left square; White POV puts rank 8 there, Black POV puts rank 1.
pub(super) fn grid_position(file: u8, rank: u8, orientation: Color) -> (u8, u8) {
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
        grid::draw_squares(&mut out, opts);
        for square in &opts.highlight {
            grid::draw_overlay(&mut out, *square, opts, &opts.theme.highlight);
        }
        if let Some(square) = opts.check {
            grid::draw_overlay(&mut out, square, opts, &opts.theme.check);
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
            coordinates::write_coordinates(&mut out, opts);
        }
        out.push_str("</svg>");
        out
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
}
