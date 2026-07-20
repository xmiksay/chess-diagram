//! Rendering options shared by every `Renderer` impl.

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
}

impl Default for Theme {
    fn default() -> Self {
        Theme {
            light: "#f0d9b5".into(),
            dark: "#b58863".into(),
            highlight: "#cdd26a".into(),
            check: "#eb3b3b".into(),
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
#[derive(Debug, Clone, PartialEq, Eq)]
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
    }
}
