//! [`PgnError`], the `pgn` feature's error type.

use thiserror::Error;

/// Errors from walking a PGN movetext. Kept separate from [`FenError`](crate::FenError)
/// so the default build's error surface is unaffected by this feature.
#[derive(Debug, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum PgnError {
    /// The `FEN` header tag (carried here, unescaped) isn't a valid position
    /// for the game's variant.
    #[error("invalid FEN tag {0:?}")]
    InvalidFen(String),
    /// The `Variant` header tag (carried here, unescaped) names a variant
    /// whose move rules differ from standard chess and Chess960.
    #[error("unsupported variant {0:?}")]
    UnsupportedVariant(String),
    /// `ply` asked for more half-moves than the mainline contains.
    #[error("PGN mainline has {available} ply, requested ply {requested}")]
    PlyOutOfRange {
        /// The ply that was requested.
        requested: usize,
        /// The number of mainline ply actually present.
        available: usize,
    },
    /// A SAN token in the mainline didn't parse.
    #[error("invalid SAN token {0:?}")]
    InvalidSan(String),
    /// A SAN token parsed but doesn't name a legal move in its position.
    #[error("illegal move {0:?}")]
    IllegalMove(String),
}
