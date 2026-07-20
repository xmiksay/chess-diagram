//! Annotation shapes drawn on top of the board (circles, arrows, text
//! badges). A rendering concern, not part of the parsed position — hence a
//! separate module from `board.rs`, which never mentions rendering.

/// A single annotation, mirroring chessground's `Shape` so a `study_set_shapes`
/// payload forwards with no translation. `orig`/`dest`/`brush` match
/// chessground verbatim; `text`/`arrow` are additive with defaults.
///
/// # Example
///
/// ```
/// use chess_diagram::{ArrowShape, Shape};
///
/// let circle = Shape {
///     orig: "e4".into(),
///     dest: None,
///     brush: "green".into(),
///     text: None,
///     arrow: ArrowShape::default(),
/// };
/// assert_eq!(circle.arrow, ArrowShape::Auto);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shape {
    /// Origin square in algebraic notation, e.g. `"e4"`.
    pub orig: String,
    /// Destination square in algebraic notation. `None` draws a circle/text
    /// badge on `orig`; `Some` draws an arrow from `orig` to `dest`.
    pub dest: Option<String>,
    /// Brush color: a named brush (`"green"`/`"red"`/`"blue"`/`"yellow"`) or
    /// a literal `"#rrggbb"`. Resolved against `Theme` at render time; an
    /// unknown name falls back to green.
    pub brush: String,
    /// Optional corner badge text drawn on `orig`.
    pub text: Option<String>,
    /// Arrow head/body style; only meaningful when `dest` is `Some`.
    pub arrow: ArrowShape,
}

/// How an arrow shape is drawn. `Auto` (the default) picks straight vs.
/// elbow from the geometry between `orig` and `dest`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ArrowShape {
    /// Pick straight vs. elbow automatically from the origin/destination
    /// geometry (e.g. a knight-move offset gets an elbow).
    #[default]
    Auto,
    /// Always draw a straight line.
    Straight,
    /// Always draw an elbowed (bent) line.
    Elbow,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arrow_shape_defaults_to_auto() {
        assert_eq!(ArrowShape::default(), ArrowShape::Auto);
    }

    #[test]
    fn shape_holds_the_fields_verbatim() {
        let shape = Shape {
            orig: "e4".into(),
            dest: Some("e5".into()),
            brush: "red".into(),
            text: Some("!".into()),
            arrow: ArrowShape::Straight,
        };
        assert_eq!(shape.orig, "e4");
        assert_eq!(shape.dest.as_deref(), Some("e5"));
        assert_eq!(shape.brush, "red");
        assert_eq!(shape.text.as_deref(), Some("!"));
        assert_eq!(shape.arrow, ArrowShape::Straight);
    }
}
