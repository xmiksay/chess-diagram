//! Per-square text badges (`Shape.text`), drawn in the top-right corner of
//! `orig` — eval/NAG-style annotations (`!`, `?`, `±`, `#1`, `+3.2`).
//! Rendered last, after the arrows, so a badge sits in the top layer and
//! stays legible over both pieces and arrow shafts. A shape's `text` is
//! independent of its `dest`: a shape can carry an arrow and a badge at
//! once. A shape with an unresolvable `orig` is silently skipped — `render`
//! stays infallible.

use std::fmt::Write;

use crate::annotation::Shape;
use crate::board::Square;
use crate::options::Options;

use super::{grid_position, SQUARE};

/// Badge text font size, in SVG units.
const FONT_SIZE: u32 = 16;
/// Inset between the badge and the square's top/right edges, in SVG units.
const INSET: u32 = 4;

/// Draws each `Shape.text` in `opts.shapes` as a badge on its `orig` square,
/// orientation-aware via [`grid_position`].
pub(super) fn draw_text_badges(out: &mut String, opts: &Options) {
    for shape in &opts.shapes {
        draw_badge(out, shape, opts);
    }
}

fn draw_badge(out: &mut String, shape: &Shape, opts: &Options) {
    let Some(text) = &shape.text else { return };
    let Some(square) = Square::from_algebraic(&shape.orig) else {
        return;
    };
    let color = opts.theme.resolve_brush(&shape.brush);
    let (col, row) = grid_position(square.file(), square.rank(), opts.orientation);
    let x = u32::from(col) * SQUARE + SQUARE - INSET;
    let y = u32::from(row) * SQUARE + INSET + FONT_SIZE;
    let _ = write!(
        out,
        r#"<text x="{x}" y="{y}" font-family="sans-serif" font-size="{FONT_SIZE}" font-weight="bold" fill="{color}" text-anchor="end">{}</text>"#,
        escape_xml(text),
    );
}

/// Escapes the five XML-significant characters so arbitrary `Shape.text`
/// stays valid, self-contained SVG rather than corrupting the markup or
/// letting a badge break out into a new element.
fn escape_xml(s: &str) -> String {
    let mut escaped = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            _ => escaped.push(c),
        }
    }
    escaped
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
    const EMPTY: &str = "8/8/8/8/8/8/8/8";

    fn badge(orig: &str, text: &str, brush: &str) -> Shape {
        Shape {
            orig: orig.into(),
            dest: None,
            brush: brush.into(),
            text: Some(text.into()),
            arrow: ArrowShape::default(),
            width: None,
        }
    }

    #[test]
    fn badge_renders_at_top_right_corner_white_pov() {
        let board = parse(START).expect("start position parses");
        let opts = Options {
            shapes: vec![badge("e4", "!", "green")],
            ..Options::default()
        };
        let svg = SvgRenderer.render(&board, &opts);
        // White POV: e4 -> grid (4, 4) -> square top-left (180, 180).
        assert!(svg.contains(
            r##"<text x="221" y="200" font-family="sans-serif" font-size="16" font-weight="bold" fill="#15781B" text-anchor="end">!</text>"##
        ));
    }

    #[test]
    fn badge_resolves_hex_brush_literally() {
        let board = parse(EMPTY).expect("empty board parses");
        let opts = Options {
            shapes: vec![badge("e4", "!", "#123456")],
            ..Options::default()
        };
        let svg = SvgRenderer.render(&board, &opts);
        assert!(svg.contains(r##"fill="#123456""##));
    }

    #[test]
    fn badge_unknown_brush_falls_back_to_green() {
        let board = parse(EMPTY).expect("empty board parses");
        let opts = Options {
            shapes: vec![badge("e4", "!", "purple")],
            ..Options::default()
        };
        let svg = SvgRenderer.render(&board, &opts);
        assert!(svg.contains(r##"fill="#15781B""##));
    }

    #[test]
    fn badge_with_unresolvable_orig_is_skipped() {
        let board = parse(EMPTY).expect("empty board parses");
        let opts = Options {
            shapes: vec![badge("z9", "!", "green")],
            coordinates: false,
            ..Options::default()
        };
        let svg = SvgRenderer.render(&board, &opts);
        assert!(!svg.contains("<text"));
    }

    #[test]
    fn badge_renders_alongside_an_arrow_on_the_same_shape() {
        let board = parse(EMPTY).expect("empty board parses");
        let shape = Shape {
            orig: "e2".into(),
            dest: Some("e4".into()),
            brush: "green".into(),
            text: Some("+3.2".into()),
            arrow: ArrowShape::Straight,
            width: None,
        };
        let svg = SvgRenderer.render(
            &board,
            &Options {
                shapes: vec![shape],
                ..Options::default()
            },
        );
        assert!(svg.contains("<line"));
        assert!(svg.contains(">+3.2</text>"));
    }

    #[test]
    fn badge_escapes_xml_special_characters() {
        let board = parse(EMPTY).expect("empty board parses");
        let opts = Options {
            shapes: vec![badge("e4", "<a>&\"'", "green")],
            ..Options::default()
        };
        let svg = SvgRenderer.render(&board, &opts);
        assert!(svg.contains(">&lt;a&gt;&amp;&quot;&apos;</text>"));
        assert!(!svg.contains("<a>"));
    }

    #[test]
    fn badge_is_drawn_above_pieces_and_arrows() {
        let board = parse("8/8/8/8/8/8/8/4K3").expect("board parses");
        let opts = Options {
            shapes: vec![
                Shape {
                    orig: "e1".into(),
                    dest: Some("e4".into()),
                    brush: "green".into(),
                    text: None,
                    arrow: ArrowShape::Straight,
                    width: None,
                },
                badge("e1", "!", "red"),
            ],
            ..Options::default()
        };
        let svg = SvgRenderer.render(&board, &opts);
        let piece_pos = svg
            .find(r#"<g transform="translate("#)
            .expect("king present");
        let line_pos = svg.find("<line").expect("line present");
        let text_pos = svg.find("<text").expect("badge present");
        assert!(piece_pos < line_pos);
        assert!(line_pos < text_pos);
    }

    #[test]
    fn badge_flips_with_orientation() {
        let board = parse(EMPTY).expect("empty board parses");
        let white = SvgRenderer.render(
            &board,
            &Options {
                shapes: vec![badge("e4", "!", "green")],
                ..Options::default()
            },
        );
        let black = SvgRenderer.render(
            &board,
            &Options {
                orientation: Color::Black,
                shapes: vec![badge("e4", "!", "green")],
                ..Options::default()
            },
        );
        assert_ne!(white, black);
    }
}
