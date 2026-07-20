//! Arrows for `Shape { dest: Some(_), .. }` — lichess-analysis style, drawn
//! above the pieces so an arrow is never hidden by the piece sitting on
//! `orig`/`dest`.
//!
//! [`ArrowShape::Straight`] and non-knight [`ArrowShape::Auto`] shapes render
//! a single straight shaft; [`ArrowShape::Elbow`] and knight-move `Auto`
//! shapes render a two-segment bent shaft (long leg then short leg, the
//! lichess knight-move convention). A shape with an unresolvable `orig`/
//! `dest` is silently skipped — `render` stays infallible.

use std::fmt::Write;

use crate::annotation::{ArrowShape, Shape};
use crate::board::{Color, Square};
use crate::options::Options;

use super::square_center;

/// Default arrow shaft stroke width, in SVG units, when `Shape::width` is
/// `None` — chessground's `lineWidth` (10) mapped onto our 45-unit square
/// (`10 / 64 × 45 ≈ 7`). The arrowhead marker scales with this via
/// `markerUnits="strokeWidth"`, so the head stays proportional at any width.
const DEFAULT_ARROW_WIDTH: f32 = 7.0;
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
        // markerUnits defaults to "strokeWidth", so these 4×4 units are 4×
        // the shaft wide and 3× long — chessground's arrowhead proportions,
        // auto-scaling with each arrow's own stroke-width.
        let _ = write!(
            out,
            r#"<marker id="{}{index}" markerWidth="4" markerHeight="4" refX="2.05" refY="2" orient="auto-start-reverse" viewBox="0 0 4 4"><polygon points="0,0 3,2 0,4" fill="{color}"/></marker>"#,
            MARKER_PREFIX
        );
    }
    out.push_str("</defs>");
}

/// Draws each renderable arrow's shaft — a `<line>` for a straight shaft, a
/// `<polyline>` for an elbowed one — with a `marker-end` pointing at the
/// matching `<defs>` entry. Call after the pieces so arrows sit on top of
/// them.
pub(super) fn draw_arrows(out: &mut String, opts: &Options) {
    let colors = arrow_colors(opts);
    for (shape, orig, dest, geometry) in renderable_arrows(opts) {
        let color = opts.theme.resolve_brush(&shape.brush);
        let index = colors
            .iter()
            .position(|c| *c == color)
            .expect("color was collected into `colors` by the same filter");
        let width = shape.width.unwrap_or(DEFAULT_ARROW_WIDTH);
        let arrow = match geometry {
            Geometry::Straight => draw_straight_arrow(orig, dest, color, index, width, opts),
            Geometry::Elbow => draw_elbow_arrow(orig, dest, color, index, width, opts),
        };
        let Some(arrow) = arrow else { continue };
        emit_arrow(out, &arrow, opts.arrow_opacity);
    }
}

/// Writes an arrow element, wrapping it in a `<g opacity="…">` when
/// `opacity < 1.0` so the shaft and its arrowhead marker composite as a single
/// translucent layer (chessground's per-shape group opacity) — a bare element
/// with separate stroke/fill opacity would double-darken where the head
/// overlaps the shaft. At full opacity the element is emitted as-is.
fn emit_arrow(out: &mut String, element: &str, opacity: f32) {
    if opacity < 1.0 {
        let _ = write!(out, r#"<g opacity="{opacity}">{element}</g>"#);
    } else {
        out.push_str(element);
    }
}

/// Trims `len` units off the `(x, y)` -> `(tx, ty)` segment's start, per the
/// unit direction of that segment — shared by the straight shaft's tail trim
/// and the elbow's tail/corner trims.
fn trim_from(x: f32, y: f32, tx: f32, ty: f32, len: f32) -> (f32, f32) {
    let (vx, vy) = (tx - x, ty - y);
    let seg_len = vx.hypot(vy);
    if seg_len <= f32::EPSILON {
        return (x, y);
    }
    (x + vx / seg_len * len, y + vy / seg_len * len)
}

fn draw_straight_arrow(
    orig: Square,
    dest: Square,
    color: &str,
    marker_index: usize,
    width: f32,
    opts: &Options,
) -> Option<String> {
    let (ox, oy) = square_center(orig, opts.orientation);
    let (dx, dy) = square_center(dest, opts.orientation);
    let (vx, vy) = (dx - ox, dy - oy);
    let len = vx.hypot(vy);
    if len <= HEAD_TRIM + TAIL_TRIM {
        return None;
    }
    let (x1, y1) = trim_from(ox, oy, dx, dy, TAIL_TRIM);
    let (x2, y2) = trim_from(dx, dy, ox, oy, HEAD_TRIM);
    let mut out = String::new();
    let _ = write!(
        out,
        r#"<line x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" stroke="{color}" stroke-width="{width}" marker-end="url(#{MARKER_PREFIX}{marker_index})"/>"#
    );
    Some(out)
}

/// Draws a knight-move arrow as a two-segment polyline: a long leg from
/// `orig` to the elbow corner, then a short leg from the corner to `dest`,
/// the lichess convention. Tail-trimmed at `orig`, head-trimmed at `dest`;
/// the corner itself is a plain joint.
fn draw_elbow_arrow(
    orig: Square,
    dest: Square,
    color: &str,
    marker_index: usize,
    width: f32,
    opts: &Options,
) -> Option<String> {
    let (ox, oy) = square_center(orig, opts.orientation);
    let (dx, dy) = square_center(dest, opts.orientation);
    let (cx, cy) = elbow_corner(orig, dest, opts.orientation);
    let head_len = (dx - cx).hypot(dy - cy);
    if head_len <= HEAD_TRIM {
        return None;
    }
    let (x1, y1) = trim_from(ox, oy, cx, cy, TAIL_TRIM);
    let (x2, y2) = trim_from(dx, dy, cx, cy, HEAD_TRIM);
    let mut out = String::new();
    let _ = write!(
        out,
        r#"<polyline points="{x1},{y1} {cx},{cy} {x2},{y2}" fill="none" stroke="{color}" stroke-width="{width}" marker-end="url(#{MARKER_PREFIX}{marker_index})"/>"#
    );
    Some(out)
}

/// The bend point of an elbow, in SVG units: the long leg runs from `orig`
/// to this corner along whichever axis (file or rank) has the larger delta,
/// then the short leg runs from the corner to `dest` along the other axis.
/// For a knight move this is the lichess convention — e.g. g1-f3 (file
/// delta 1, rank delta 2) corners at g3, the long leg vertical.
fn elbow_corner(orig: Square, dest: Square, orientation: Color) -> (f32, f32) {
    let (ox, oy) = square_center(orig, orientation);
    let (dx, dy) = square_center(dest, orientation);
    let df = (i16::from(orig.file()) - i16::from(dest.file())).abs();
    let dr = (i16::from(orig.rank()) - i16::from(dest.rank())).abs();
    if df > dr {
        (dx, oy)
    } else {
        (ox, dy)
    }
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

/// A renderable arrow's shaft shape, resolved from `Shape::arrow` and the
/// `orig`/`dest` geometry.
#[derive(Clone, Copy)]
enum Geometry {
    Straight,
    Elbow,
}

/// Shapes that resolve to an arrow render: valid `orig`/`dest` squares, with
/// the shaft geometry `Shape::arrow` resolves to.
fn renderable_arrows(opts: &Options) -> impl Iterator<Item = (&Shape, Square, Square, Geometry)> {
    opts.shapes.iter().filter_map(|shape| {
        let dest = shape.dest.as_deref()?;
        let orig = Square::from_algebraic(&shape.orig)?;
        let dest = Square::from_algebraic(dest)?;
        let elbow = match shape.arrow {
            ArrowShape::Straight => false,
            ArrowShape::Auto => is_knight_move(orig, dest),
            // A same-file/same-rank move has no second axis to bend
            // around, so an elbow degenerates to a straight shaft anyway.
            ArrowShape::Elbow => orig.file() != dest.file() && orig.rank() != dest.rank(),
        };
        let geometry = if elbow {
            Geometry::Elbow
        } else {
            Geometry::Straight
        };
        Some((shape, orig, dest, geometry))
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
            width: None,
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
    fn arrow_uses_default_shaft_width_when_unset() {
        let board = parse(EMPTY).expect("empty board parses");
        let opts = Options {
            shapes: vec![arrow("a1", "a8", "green", ArrowShape::Straight)],
            ..Options::default()
        };
        let svg = SvgRenderer.render(&board, &opts);
        assert!(svg.contains(r#"stroke-width="7""#));
    }

    #[test]
    fn per_shape_width_overrides_the_default() {
        let board = parse(EMPTY).expect("empty board parses");
        let mut shape = arrow("a1", "a8", "green", ArrowShape::Straight);
        shape.width = Some(12.5);
        let opts = Options {
            shapes: vec![shape],
            ..Options::default()
        };
        let svg = SvgRenderer.render(&board, &opts);
        assert!(svg.contains(r#"stroke-width="12.5""#));
        assert!(!svg.contains(r#"stroke-width="7""#));
    }

    #[test]
    fn arrow_opacity_below_one_wraps_the_element_in_a_group() {
        let board = parse(EMPTY).expect("empty board parses");
        let opts = Options {
            shapes: vec![arrow("a1", "a8", "green", ArrowShape::Straight)],
            arrow_opacity: 0.4,
            ..Options::default()
        };
        let svg = SvgRenderer.render(&board, &opts);
        assert!(svg.contains(r#"<g opacity="0.4"><line"#));
    }

    #[test]
    fn full_opacity_emits_a_bare_element() {
        let board = parse(EMPTY).expect("empty board parses");
        let opts = Options {
            shapes: vec![arrow("a1", "a8", "green", ArrowShape::Straight)],
            ..Options::default()
        };
        let svg = SvgRenderer.render(&board, &opts);
        assert!(!svg.contains(r#"<g opacity"#));
    }

    #[test]
    fn transparent_hex_brush_passes_through_to_stroke_and_marker() {
        let board = parse(EMPTY).expect("empty board parses");
        let opts = Options {
            shapes: vec![arrow("a1", "a8", "#15781Bcc", ArrowShape::Straight)],
            ..Options::default()
        };
        let svg = SvgRenderer.render(&board, &opts);
        assert!(svg.contains(r##"stroke="#15781Bcc""##));
        assert!(svg.contains(r##"fill="#15781Bcc""##));
    }

    #[test]
    fn auto_arrow_on_a_knight_move_renders_an_elbow() {
        let board = parse(EMPTY).expect("empty board parses");
        let opts = Options {
            shapes: vec![arrow("g1", "f3", "green", ArrowShape::Auto)],
            ..Options::default()
        };
        let svg = SvgRenderer.render(&board, &opts);
        assert!(!svg.contains("<line"));
        assert!(svg.contains("<polyline"));
        assert!(svg.contains("<marker"));
        // g1 (292.5, 337.5) -> corner (292.5, 247.5) -> f3 (247.5, 247.5),
        // long leg vertical then short leg horizontal, tail/head-trimmed.
        assert!(svg.contains(r#"points="292.5,325.5 292.5,247.5 267.5,247.5""#));
        assert!(svg.contains("marker-end=\"url(#chess-diagram-arrowhead-0)\""));
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
    fn elbow_style_forces_a_bent_shaft_on_a_non_knight_move() {
        let board = parse(EMPTY).expect("empty board parses");
        let opts = Options {
            shapes: vec![arrow("a1", "d4", "green", ArrowShape::Elbow)],
            ..Options::default()
        };
        let svg = SvgRenderer.render(&board, &opts);
        assert!(svg.contains("<polyline"));
        assert!(!svg.contains("<line"));
    }

    #[test]
    fn elbow_style_on_a_straight_line_move_degenerates_to_a_straight_shaft() {
        // a1-a8 has no second axis to bend around (same file), so forcing
        // `Elbow` still renders a plain straight shaft.
        let board = parse(EMPTY).expect("empty board parses");
        let opts = Options {
            shapes: vec![arrow("a1", "a8", "green", ArrowShape::Elbow)],
            ..Options::default()
        };
        let svg = SvgRenderer.render(&board, &opts);
        assert!(svg.contains("<line"));
        assert!(!svg.contains("<polyline"));
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

    #[test]
    fn elbow_arrow_flips_with_orientation() {
        let board = parse(EMPTY).expect("empty board parses");
        let white = SvgRenderer.render(
            &board,
            &Options {
                shapes: vec![arrow("g1", "f3", "green", ArrowShape::Auto)],
                ..Options::default()
            },
        );
        let black = SvgRenderer.render(
            &board,
            &Options {
                orientation: Color::Black,
                shapes: vec![arrow("g1", "f3", "green", ArrowShape::Auto)],
                ..Options::default()
            },
        );
        assert_ne!(white, black);
    }

    #[test]
    fn is_knight_move_detects_all_eight_offsets_and_rejects_others() {
        let orig = Square::from_algebraic("d4").expect("valid square");
        let knight_offsets = [
            (1, 2),
            (2, 1),
            (-1, 2),
            (-2, 1),
            (1, -2),
            (2, -1),
            (-1, -2),
            (-2, -1),
        ];
        for (df, dr) in knight_offsets {
            let dest = Square::new((3 + df) as u8, (3 + dr) as u8).expect("valid square");
            assert!(is_knight_move(orig, dest), "({df}, {dr}) is a knight move");
        }
        let non_knight_offsets = [(0, 1), (1, 1), (1, 0), (2, 2), (0, 2), (3, 1)];
        for (df, dr) in non_knight_offsets {
            let dest = Square::new((3 + df) as u8, (3 + dr) as u8).expect("valid square");
            assert!(
                !is_knight_move(orig, dest),
                "({df}, {dr}) is not a knight move"
            );
        }
    }
}
