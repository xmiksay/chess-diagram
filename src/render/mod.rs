//! The renderer seam. Every output format is one impl of [`Renderer`];
//! adding a format (PNG, typst) is an additive change here, never a rewrite
//! of the parse layer.

use crate::board::Board;
use crate::options::{Format, Options};

/// One output format = one impl. Modeled on mdcast's `Backend` trait,
/// minus async and asset plumbing — this crate is a pure function.
pub trait Renderer {
    fn format(&self) -> Format;
    fn render(&self, board: &Board, opts: &Options) -> String;
}

#[cfg(feature = "svg")]
mod svg;
#[cfg(feature = "svg")]
pub use svg::SvgRenderer;
