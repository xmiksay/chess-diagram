//! Render a chess **position** (a FEN string) to a static, self-contained
//! **SVG** diagram — no client JS, no native dependencies.
//!
//! # Quick start
//!
//! ```
//! let svg = chess_diagram::render_svg(
//!     "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
//!     &Default::default(),
//! )?;
//! assert!(svg.starts_with("<svg"));
//! # Ok::<(), chess_diagram::FenError>(())
//! ```
//!
//! # Options
//!
//! [`Options`] controls board orientation, coordinate labels, size, theme
//! colors, which squares get a highlight/check overlay, and which
//! [`Shape`] annotations (circles, straight/elbowed arrows, and text
//! badges) are drawn:
//!
//! ```
//! use chess_diagram::{Color, Options, Square};
//!
//! let opts = Options {
//!     orientation: Color::Black,
//!     highlight: vec![
//!         Square::from_algebraic("e2").unwrap(),
//!         Square::from_algebraic("e4").unwrap(),
//!     ],
//!     ..Default::default()
//! };
//! let svg = chess_diagram::render_svg(
//!     "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
//!     &opts,
//! )?;
//! assert!(svg.starts_with("<svg"));
//! # Ok::<(), chess_diagram::FenError>(())
//! ```
//!
//! # Architecture
//!
//! The core is a pure function `FEN → String`, split at one seam. Parsing
//! ([`parse`]) and the position model ([`Board`]/[`Piece`]/[`Square`]) know
//! nothing about output formats — no format name and no SVG string ever
//! appears there; every output format is instead one impl of the
//! [`Renderer`] trait ([`SvgRenderer`] ships in v1), so adding a
//! string-producing format (typst) is additive at that seam rather than a
//! change to parsing or the model. PNG rasterises `SvgRenderer`'s own output
//! instead of being a second `Renderer` impl, since it produces bytes rather
//! than a `String` (see `png::render_png`).
//!
//! # Features
//!
//! - `svg` (default) — [`SvgRenderer`] and [`render_svg`]. Pure Rust, no
//!   dependencies beyond `thiserror`.
//! - `pgn` (opt-in) — `pgn::board_at` walks a PGN mainline to a given ply
//!   via `shakmaty`. Not part of the default build's dependency tree.
//! - `png` (opt-in, implies `svg`) — `png::render_png` rasterises this
//!   crate's own SVG output to PNG bytes via `resvg`. Not part of the
//!   default build's dependency tree.
#![warn(missing_docs)]

mod annotation;
mod board;
mod fen;
mod options;
#[cfg(feature = "pgn")]
pub mod pgn;
#[cfg(feature = "svg")]
mod pieces;
#[cfg(feature = "png")]
pub mod png;
mod render;

pub use annotation::{ArrowShape, Shape};
pub use board::{Board, Color, Piece, Role, Square};
pub use fen::{parse, FenError};
pub use options::{Format, Options, Theme};
pub use render::Renderer;
#[cfg(feature = "svg")]
pub use render::SvgRenderer;

/// One-liner convenience: parse `fen` and render it with [`SvgRenderer`].
///
/// # Example
///
/// ```
/// use chess_diagram::{render_svg, Options};
///
/// let svg = render_svg("8/8/8/8/8/8/8/4K3", &Options::default())?;
/// assert!(svg.starts_with("<svg"));
/// # Ok::<(), chess_diagram::FenError>(())
/// ```
#[cfg(feature = "svg")]
pub fn render_svg(fen: &str, opts: &Options) -> Result<String, FenError> {
    let board = parse(fen)?;
    Ok(SvgRenderer.render(&board, opts))
}
