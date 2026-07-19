//! Hand-emitted SVG renderer — strings only, no dependencies.
//!
//! Phase 0 stub: emits the board grid (orientation- and theme-aware) as a
//! valid standalone SVG. Pieces, coordinates, and highlight/check squares
//! land in Phase 1 against the golden-test harness.

use std::fmt::Write;

use crate::board::{Board, Color};
use crate::options::{Format, Options};
use crate::render::Renderer;

pub struct SvgRenderer;

impl Renderer for SvgRenderer {
    fn format(&self) -> Format {
        Format::Svg
    }

    fn render(&self, _board: &Board, opts: &Options) -> String {
        let size = opts.size;
        let mut out = String::with_capacity(4096);
        let _ = write!(
            out,
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{size}" height="{size}" viewBox="0 0 8 8">"#
        );
        for rank in 0..8u8 {
            for file in 0..8u8 {
                let (x, y) = match opts.orientation {
                    Color::White => (file, 7 - rank),
                    Color::Black => (7 - file, rank),
                };
                let fill = if (file + rank) % 2 == 0 {
                    &opts.theme.dark
                } else {
                    &opts.theme.light
                };
                let _ = write!(
                    out,
                    r#"<rect x="{x}" y="{y}" width="1" height="1" fill="{fill}"/>"#
                );
            }
        }
        out.push_str("</svg>");
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fen::parse;

    const START: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

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
    fn a1_is_dark() {
        let board = parse(START).expect("start position parses");
        let opts = Options::default();
        let svg = SvgRenderer.render(&board, &opts);
        // White POV: a1 sits bottom-left at grid (0, 7).
        let a1 = format!(
            r#"<rect x="0" y="7" width="1" height="1" fill="{}"/>"#,
            opts.theme.dark
        );
        assert!(svg.contains(&a1));
    }
}
