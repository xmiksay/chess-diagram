//! Single-square annotation shapes (`Options::shapes`), drawn between the
//! highlight/check overlays and the pieces so a shape never hides the piece
//! sitting on it. Arrows (`Shape::dest` set) render above the pieces
//! instead — see `super::arrows`.
//!
//! Only circles (`dest: None`, `text: None`) render here; text badges are a
//! later shape kind. A shape with an unresolvable `orig`, or one that isn't
//! a circle, is silently skipped — `render` stays infallible.

use std::fmt::Write;

use crate::annotation::Shape;
use crate::board::Square;
use crate::options::Options;

use super::{square_center, SQUARE};

/// Circle stroke width, in SVG units.
const STROKE_WIDTH: f32 = 3.0;
/// Inset between the circle's stroke and the square edge, in SVG units.
const INSET: f32 = 3.0;

/// Draws each shape in `opts.shapes`, orientation-aware via [`square_center`].
pub(super) fn draw_shapes(out: &mut String, opts: &Options) {
    for shape in &opts.shapes {
        draw_circle(out, shape, opts);
    }
}

fn draw_circle(out: &mut String, shape: &Shape, opts: &Options) {
    if shape.dest.is_some() || shape.text.is_some() {
        return;
    }
    let Some(square) = Square::from_algebraic(&shape.orig) else {
        return;
    };
    let color = opts.theme.resolve_brush(&shape.brush);
    let (cx, cy) = square_center(square, opts.orientation);
    let radius = SQUARE as f32 / 2.0 - INSET;
    let _ = write!(
        out,
        r#"<circle cx="{cx}" cy="{cy}" r="{radius}" fill="none" stroke="{color}" stroke-width="{STROKE_WIDTH}"/>"#
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::annotation::ArrowShape;
    use crate::board::Color;
    use crate::fen::parse;
    use crate::render::svg::SvgRenderer;
    use crate::render::Renderer;

    const START: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    // Piece glyphs (`pieces.rs`) already use `<circle>` for eyes/pommels, so
    // the "no circle rendered" tests use an empty board to avoid a false
    // positive from a piece glyph rather than a shape.
    const EMPTY: &str = "8/8/8/8/8/8/8/8";

    fn circle(orig: &str, brush: &str) -> Shape {
        Shape {
            orig: orig.into(),
            dest: None,
            brush: brush.into(),
            text: None,
            arrow: ArrowShape::default(),
        }
    }

    #[test]
    fn circle_renders_at_square_center_white_pov() {
        let board = parse(START).expect("start position parses");
        let opts = Options {
            shapes: vec![circle("e4", "green")],
            ..Options::default()
        };
        let svg = SvgRenderer.render(&board, &opts);
        // White POV: e4 -> grid (4, 4) -> center (202.5, 202.5).
        assert!(svg.contains(
            r##"<circle cx="202.5" cy="202.5" r="19.5" fill="none" stroke="#15781B" stroke-width="3""##
        ));
    }

    #[test]
    fn circle_resolves_hex_brush_literally() {
        let board = parse(START).expect("start position parses");
        let opts = Options {
            shapes: vec![circle("e4", "#123456")],
            ..Options::default()
        };
        let svg = SvgRenderer.render(&board, &opts);
        assert!(svg.contains(r##"stroke="#123456""##));
    }

    #[test]
    fn circle_unknown_brush_falls_back_to_green() {
        let board = parse(START).expect("start position parses");
        let opts = Options {
            shapes: vec![circle("e4", "purple")],
            ..Options::default()
        };
        let svg = SvgRenderer.render(&board, &opts);
        assert!(svg.contains(r##"stroke="#15781B""##));
    }

    #[test]
    fn shape_with_unresolvable_orig_is_skipped() {
        let board = parse(EMPTY).expect("empty board parses");
        let opts = Options {
            shapes: vec![circle("z9", "green")],
            ..Options::default()
        };
        let svg = SvgRenderer.render(&board, &opts);
        assert!(!svg.contains("<circle"));
    }

    #[test]
    fn shape_with_dest_or_text_is_not_yet_rendered() {
        let board = parse(EMPTY).expect("empty board parses");
        let mut arrow = circle("e4", "green");
        arrow.dest = Some("e5".into());
        let mut text = circle("e4", "green");
        text.text = Some("!".into());
        let opts = Options {
            shapes: vec![arrow, text],
            ..Options::default()
        };
        let svg = SvgRenderer.render(&board, &opts);
        assert!(!svg.contains("<circle"));
    }

    #[test]
    fn circle_is_drawn_under_pieces() {
        let board = parse(START).expect("start position parses");
        let opts = Options {
            shapes: vec![circle("e2", "green")],
            ..Options::default()
        };
        let svg = SvgRenderer.render(&board, &opts);
        let circle_pos = svg.find("<circle").expect("circle present");
        let piece_pos = svg
            .find(r#"<g transform="translate("#)
            .expect("at least one piece present");
        assert!(circle_pos < piece_pos);
    }

    #[test]
    fn circle_flips_with_orientation() {
        let board = parse(START).expect("start position parses");
        let opts = Options {
            orientation: Color::Black,
            shapes: vec![circle("e4", "green")],
            ..Options::default()
        };
        let svg = SvgRenderer.render(&board, &opts);
        // Black POV: e4 -> grid (3, 3) -> center (157.5, 157.5).
        assert!(svg.contains(r#"<circle cx="157.5" cy="157.5""#));
    }
}
