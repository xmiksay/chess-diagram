//! Rendering options shared by every `Renderer` impl.

use crate::annotation::Shape;
use crate::board::{Color, Square};

/// Output format a renderer produces. `Png`/`Typst` are added here later —
/// additively, which is why the enum is non-exhaustive.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// Static, self-contained SVG.
    Svg,
}

/// Square colors. Defaults match the familiar lichess brown board; `highlight`
/// and `check` are the overlay tints drawn per `Options::highlight` and
/// `Options::check` — `Theme` is the one place square colors live, so
/// renderers never hard-code a color.
///
/// # Example
///
/// ```
/// use chess_diagram::{Options, Theme};
///
/// let opts = Options {
///     theme: Theme {
///         light: "#eeeeee".into(),
///         dark: "#555555".into(),
///         ..Theme::default()
///     },
///     ..Default::default()
/// };
/// assert_eq!(opts.theme.light, "#eeeeee");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Theme {
    /// Light square fill color.
    pub light: String,
    /// Dark square fill color.
    pub dark: String,
    /// Overlay tint for `Options::highlight` squares (e.g. last-move from/to).
    pub highlight: String,
    /// Overlay tint for the `Options::check` square.
    pub check: String,
    /// Named brush: `Shape::brush == "green"`.
    pub green: String,
    /// Named brush: `Shape::brush == "red"`.
    pub red: String,
    /// Named brush: `Shape::brush == "blue"`.
    pub blue: String,
    /// Named brush: `Shape::brush == "yellow"`.
    pub yellow: String,
}

impl Default for Theme {
    fn default() -> Self {
        Theme {
            light: "#f0d9b5".into(),
            dark: "#b58863".into(),
            highlight: "#cdd26a".into(),
            check: "#eb3b3b".into(),
            green: "#15781B".into(),
            red: "#882020".into(),
            blue: "#003088".into(),
            yellow: "#e68f00".into(),
        }
    }
}

impl Theme {
    /// Resolves a [`Shape::brush`](crate::Shape::brush) value to a concrete
    /// color: a `#`-prefixed value is used literally, a known brush name
    /// (`"green"`/`"red"`/`"blue"`/`"yellow"`) looks up the matching field
    /// here, and anything else falls back to green — matching chess-base's
    /// "unknown brush → green".
    pub fn resolve_brush<'a>(&'a self, brush: &'a str) -> &'a str {
        if brush.starts_with('#') {
            return brush;
        }
        match brush {
            "red" => &self.red,
            "blue" => &self.blue,
            "yellow" => &self.yellow,
            _ => &self.green,
        }
    }
}

/// How to draw the diagram; `Options::default()` is a sensible board.
///
/// # Example
///
/// Flip the board to Black's point of view and highlight a last move:
///
/// ```
/// use chess_diagram::{Color, Options, Square};
///
/// let opts = Options {
///     orientation: Color::Black,
///     highlight: vec![
///         Square::from_algebraic("e2").unwrap(),
///         Square::from_algebraic("e4").unwrap(),
///     ],
///     ..Default::default()
/// };
/// let svg = chess_diagram::render_svg(
///     "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
///     &opts,
/// )?;
/// assert!(svg.starts_with("<svg"));
/// # Ok::<(), chess_diagram::FenError>(())
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Options {
    /// Board point of view (default: White at the bottom).
    pub orientation: Color,
    /// Squares to highlight, e.g. last-move from/to.
    pub highlight: Vec<Square>,
    /// King-in-check square, drawn with a warning tint.
    pub check: Option<Square>,
    /// Draw file/rank labels.
    pub coordinates: bool,
    /// Rendered edge length in SVG px.
    pub size: u32,
    /// Square and overlay colors.
    pub theme: Theme,
    /// Annotation shapes: circles draw under the pieces, straight arrows
    /// draw above them (text badges are a later shape kind).
    pub shapes: Vec<Shape>,
    /// Opacity applied to every arrow (shaft + head together), `0.0`–`1.0`.
    /// `1.0` (the default) is fully opaque, matching chessground's solid
    /// brushes; lower it toward `0.4` for the "pale" translucent look.
    pub arrow_opacity: f32,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            orientation: Color::White,
            highlight: Vec::new(),
            check: None,
            coordinates: true,
            size: 360,
            theme: Theme::default(),
            shapes: Vec::new(),
            arrow_opacity: 1.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_options_are_sane() {
        let opts = Options::default();
        assert_eq!(opts.orientation, Color::White);
        assert!(opts.highlight.is_empty());
        assert_eq!(opts.check, None);
        assert!(opts.coordinates);
        assert!(opts.size > 0);
        assert!(!opts.theme.highlight.is_empty());
        assert!(!opts.theme.check.is_empty());
        assert!(opts.shapes.is_empty());
        assert_eq!(opts.arrow_opacity, 1.0);
    }

    #[test]
    fn resolve_brush_looks_up_named_colors() {
        let theme = Theme::default();
        assert_eq!(theme.resolve_brush("green"), theme.green);
        assert_eq!(theme.resolve_brush("red"), theme.red);
        assert_eq!(theme.resolve_brush("blue"), theme.blue);
        assert_eq!(theme.resolve_brush("yellow"), theme.yellow);
    }

    #[test]
    fn resolve_brush_uses_hex_literally() {
        let theme = Theme::default();
        assert_eq!(theme.resolve_brush("#123456"), "#123456");
    }

    #[test]
    fn resolve_brush_unknown_name_falls_back_to_green() {
        let theme = Theme::default();
        assert_eq!(theme.resolve_brush("purple"), theme.green);
        assert_eq!(theme.resolve_brush(""), theme.green);
    }
}
