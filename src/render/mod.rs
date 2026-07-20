//! The renderer seam. Every output format is one impl of [`Renderer`];
//! adding a format (PNG, typst) is an additive change here, never a rewrite
//! of the parse layer.

use crate::board::Board;
use crate::options::{Format, Options};

/// One output format = one impl. Modeled on mdcast's `Backend` trait,
/// minus async and asset plumbing — this crate is a pure function.
///
/// # Example
///
/// ```
/// use chess_diagram::{parse, Options, Renderer, SvgRenderer};
///
/// let board = parse("8/8/8/8/8/8/8/4K3")?;
/// let svg = SvgRenderer.render(&board, &Options::default());
/// assert!(svg.starts_with("<svg"));
/// # Ok::<(), chess_diagram::FenError>(())
/// ```
pub trait Renderer {
    /// The output format this impl produces.
    fn format(&self) -> Format;
    /// Renders `board` under `opts` to this impl's output format.
    fn render(&self, board: &Board, opts: &Options) -> String;
}

#[cfg(feature = "svg")]
mod svg;
#[cfg(feature = "svg")]
pub use svg::SvgRenderer;
