//! Render a chess **position** (a FEN string) to a static, self-contained
//! **SVG** diagram — no client JS, no native dependencies.
//!
//! The core is a pure function `FEN → String`. Parsing ([`parse`]) and the
//! position model ([`Board`]) know nothing about output formats; every format
//! is one impl of the [`Renderer`] trait, so new formats (PNG, typst) are
//! additive at that seam.
//!
//! # Example
//!
//! ```
//! let svg = chess_diagram::render_svg(
//!     "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
//!     &Default::default(),
//! )?;
//! assert!(svg.starts_with("<svg"));
//! # Ok::<(), chess_diagram::FenError>(())
//! ```

mod board;
mod fen;
mod options;
mod render;

pub use board::{Board, Color, Piece, Role, Square};
pub use fen::{parse, FenError};
pub use options::{Format, Options, Theme};
pub use render::Renderer;
#[cfg(feature = "svg")]
pub use render::SvgRenderer;

/// One-liner convenience: parse `fen` and render it with [`SvgRenderer`].
#[cfg(feature = "svg")]
pub fn render_svg(fen: &str, opts: &Options) -> Result<String, FenError> {
    let board = parse(fen)?;
    Ok(SvgRenderer.render(&board, opts))
}
