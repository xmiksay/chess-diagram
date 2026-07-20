//! Straight arrows for `Shape { dest: Some(_), .. }` — lichess-analysis
//! style, drawn above the pieces so an arrow is never hidden by the piece
//! sitting on `orig`/`dest`.
//!
//! Only [`ArrowShape::Straight`] and non-knight [`ArrowShape::Auto`] shapes
//! render here; a knight-move `Auto` shape and `ArrowShape::Elbow` want a
//! bent shaft, a later shape kind. A shape with an unresolvable `orig`/
//! `dest` is silently skipped — `render` stays infallible.

use std::fmt::Write;

use crate::annotation::{ArrowShape, Shape};
use crate::board::Square;
use crate::options::Options;

use super::square_center;

/// Arrow shaft stroke width, in SVG units.
const STROKE_WIDTH: f32 = 6.0;
/// How far the tail is pulled in from `orig`'s center, in SVG units.
const TAIL_TRIM: f32 = 12.0;
/// How far the head is pulled in from `dest`'s center, in SVG units — leaves
/// room for the arrowhead marker to sit inside the destination square.
const HEAD_TRIM: f32 = 20.0;
/// Marker `id` prefix, namespaced so an embedding page's own `<marker>` ids
/// can't collide with these.
const MARKER_PREFIX: &str = "chess-diagram-arrowhead-";

/// Writes the `<defs>` block of arrowhead markers, one per distinct brush
/// color among the renderable arrows — emitted right after the `<svg>`
/// header, before anything else, per the SVG spec's forward-reference rule
/// for `<marker>`. Emits nothing when there are no arrows to draw.
pub(super) fn write_defs(out: &mut String, opts: &Options) {
    let colors = arrow_colors(opts);
    if colors.is_empty() {
        return;
    }
    out.push_str("<defs>");
    for (index, color) in colors.iter().enumerate() {
        let _ = write!(
            out,
            r#"<marker id="{}{index}" markerWidth="10" markerHeight="10" refX="9" refY="5" orient="auto-start-reverse" viewBox="0 0 10 10"><polygon points="0 0, 10 5, 0 10" fill="{color}"/></marker>"#,
            MARKER_PREFIX
        );
    }
    out.push_str("</defs>");
}

/// Draws each renderable arrow's shaft as a `<line>` with a `marker-end`
/// pointing at the matching `<defs>` entry. Call after the pieces so arrows
/// sit on top of them.
pub(super) fn draw_arrows(out: &mut String, opts: &Options) {
    let colors = arrow_colors(opts);
    for (shape, orig, dest) in renderable_arrows(opts) {
        let color = opts.theme.resolve_brush(&shape.brush);
        let index = colors
            .iter()
            .position(|c| *c == color)
            .expect("color was collected into `colors` by the same filter");
        draw_arrow(out, orig, dest, color, index, opts);
    }
}

fn draw_arrow(
    out: &mut String,
    orig: Square,
    dest: Square,
    color: &str,
    marker_index: usize,
    opts: &Options,
) {
    let (ox, oy) = square_center(orig, opts.orientation);
    let (dx, dy) = square_center(dest, opts.orientation);
    let (vx, vy) = (dx - ox, dy - oy);
    let len = vx.hypot(vy);
    if len <= HEAD_TRIM + TAIL_TRIM {
        return;
    }
    let (ux, uy) = (vx / len, vy / len);
    let (x1, y1) = (ox + ux * TAIL_TRIM, oy + uy * TAIL_TRIM);
    let (x2, y2) = (dx - ux * HEAD_TRIM, dy - uy * HEAD_TRIM);
    let _ = write!(
        out,
        r#"<line x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" stroke="{color}" stroke-width="{STROKE_WIDTH}" marker-end="url(#{MARKER_PREFIX}{marker_index})"/>"#
    );
}

/// Distinct brush colors among the renderable arrows, in first-appearance
/// order — the index into this list is the marker id both [`write_defs`]
/// and [`draw_arrows`] key off of.
fn arrow_colors(opts: &Options) -> Vec<&str> {
    let mut colors = Vec::new();
    for (shape, ..) in renderable_arrows(opts) {
        let color = opts.theme.resolve_brush(&shape.brush);
        if !colors.contains(&color) {
            colors.push(color);
        }
    }
    colors
}

/// Shapes that resolve to a straight-arrow render: valid `orig`/`dest`
/// squares and an arrow style that wants a straight shaft.
fn renderable_arrows(opts: &Options) -> impl Iterator<Item = (&Shape, Square, Square)> {
    opts.shapes.iter().filter_map(|shape| {
        let dest = shape.dest.as_deref()?;
        let orig = Square::from_algebraic(&shape.orig)?;
        let dest = Square::from_algebraic(dest)?;
        let straight = match shape.arrow {
            ArrowShape::Straight => true,
            ArrowShape::Auto => !is_knight_move(orig, dest),
            ArrowShape::Elbow => false,
        };
        straight.then_some((shape, orig, dest))
    })
}

/// Whether `orig` -> `dest` is a knight-move offset (file/rank deltas of
/// `(1, 2)` or `(2, 1)`) — `Auto` picks an elbow for these, not a straight
/// shaft.
fn is_knight_move(orig: Square, dest: Square) -> bool {
    let df = (i16::from(orig.file()) - i16::from(dest.file())).abs();
    let dr = (i16::from(orig.rank()) - i16::from(dest.rank())).abs();
    (df, dr) == (1, 2) || (df, dr) == (2, 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::Color;
    use crate::fen::parse;
    use crate::render::svg::SvgRenderer;
    use crate::render::Renderer;

    const EMPTY: &str = "8/8/8/8/8/8/8/8";

    fn arrow(orig: &str, dest: &str, brush: &str, style: ArrowShape) -> Shape {
        Shape {
            orig: orig.into(),
            dest: Some(dest.into()),
            brush: brush.into(),
            text: None,
            arrow: style,
        }
    }

    #[test]
    fn straight_arrow_renders_a_line_and_marker() {
        let board = parse(EMPTY).expect("empty board parses");
        let opts = Options {
            shapes: vec![arrow("a1", "a8", "green", ArrowShape::Straight)],
            ..Options::default()
        };
        let svg = SvgRenderer.render(&board, &opts);
        assert!(svg.contains("<marker"));
        assert!(svg.contains(r##"fill="#15781B""##));
        assert!(svg.contains("<line"));
        assert!(svg.contains(r##"stroke="#15781B""##));
        assert!(svg.contains("marker-end=\"url(#chess-diagram-arrowhead-0)\""));
    }

    #[test]
    fn auto_arrow_on_a_knight_move_is_not_yet_rendered() {
        let board = parse(EMPTY).expect("empty board parses");
        let opts = Options {
            shapes: vec![arrow("g1", "f3", "green", ArrowShape::Auto)],
            ..Options::default()
        };
        let svg = SvgRenderer.render(&board, &opts);
        assert!(!svg.contains("<line"));
        assert!(!svg.contains("<marker"));
    }

    #[test]
    fn straight_arrow_forces_a_straight_shaft_even_on_a_knight_move() {
        let board = parse(EMPTY).expect("empty board parses");
        let opts = Options {
            shapes: vec![arrow("g1", "f3", "green", ArrowShape::Straight)],
            ..Options::default()
        };
        let svg = SvgRenderer.render(&board, &opts);
        assert!(svg.contains("<line"));
    }

    #[test]
    fn elbow_arrow_is_not_yet_rendered() {
        let board = parse(EMPTY).expect("empty board parses");
        let opts = Options {
            shapes: vec![arrow("a1", "a8", "green", ArrowShape::Elbow)],
            ..Options::default()
        };
        let svg = SvgRenderer.render(&board, &opts);
        assert!(!svg.contains("<line"));
    }

    #[test]
    fn arrow_with_unresolvable_square_is_skipped() {
        let board = parse(EMPTY).expect("empty board parses");
        let opts = Options {
            shapes: vec![arrow("z9", "a8", "green", ArrowShape::Straight)],
            ..Options::default()
        };
        let svg = SvgRenderer.render(&board, &opts);
        assert!(!svg.contains("<line"));
        assert!(!svg.contains("<marker"));
    }

    #[test]
    fn distinct_brush_colors_get_distinct_markers() {
        let board = parse(EMPTY).expect("empty board parses");
        let opts = Options {
            shapes: vec![
                arrow("a1", "a8", "green", ArrowShape::Straight),
                arrow("b1", "b8", "red", ArrowShape::Straight),
            ],
            ..Options::default()
        };
        let svg = SvgRenderer.render(&board, &opts);
        assert_eq!(svg.matches("<marker").count(), 2);
        assert!(svg.contains("url(#chess-diagram-arrowhead-0)"));
        assert!(svg.contains("url(#chess-diagram-arrowhead-1)"));
    }

    #[test]
    fn same_brush_color_shares_one_marker() {
        let board = parse(EMPTY).expect("empty board parses");
        let opts = Options {
            shapes: vec![
                arrow("a1", "a8", "green", ArrowShape::Straight),
                arrow("b1", "b8", "green", ArrowShape::Straight),
            ],
            ..Options::default()
        };
        let svg = SvgRenderer.render(&board, &opts);
        assert_eq!(svg.matches("<marker").count(), 1);
    }

    #[test]
    fn arrow_is_drawn_above_pieces() {
        let board = parse("8/8/8/8/8/8/8/4K3").expect("board parses");
        let opts = Options {
            shapes: vec![arrow("e1", "e4", "green", ArrowShape::Straight)],
            ..Options::default()
        };
        let svg = SvgRenderer.render(&board, &opts);
        let piece_pos = svg
            .find(r#"<g transform="translate("#)
            .expect("king present");
        let line_pos = svg.find("<line").expect("line present");
        assert!(piece_pos < line_pos);
    }

    #[test]
    fn defs_block_is_emitted_right_after_the_svg_header() {
        let board = parse(EMPTY).expect("empty board parses");
        let opts = Options {
            shapes: vec![arrow("a1", "a8", "green", ArrowShape::Straight)],
            ..Options::default()
        };
        let svg = SvgRenderer.render(&board, &opts);
        let header_end = svg.find('>').expect("svg header present") + 1;
        assert!(svg[header_end..].starts_with("<defs>"));
    }

    #[test]
    fn no_arrows_means_no_defs_block() {
        let board = parse(EMPTY).expect("empty board parses");
        let svg = SvgRenderer.render(&board, &Options::default());
        assert!(!svg.contains("<defs>"));
    }

    #[test]
    fn arrow_flips_with_orientation() {
        let board = parse(EMPTY).expect("empty board parses");
        let white = SvgRenderer.render(
            &board,
            &Options {
                shapes: vec![arrow("a1", "a8", "green", ArrowShape::Straight)],
                ..Options::default()
            },
        );
        let black = SvgRenderer.render(
            &board,
            &Options {
                orientation: Color::Black,
                shapes: vec![arrow("a1", "a8", "green", ArrowShape::Straight)],
                ..Options::default()
            },
        );
        assert_ne!(white, black);
    }
}
