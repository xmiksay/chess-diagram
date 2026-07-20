//! Sample positions shared by the golden-test harness (`tests/integration.rs`)
//! and the README gallery generator (`examples/gallery.rs`) — one sample set,
//! not two. Included by both via `#[path]`.

use chess_diagram::{Color, Options, Square};

pub const START_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
pub const MIDGAME_FEN: &str = "r1bqk2r/pp2bppp/2n2n2/2pp4/4P3/2NP1N2/PPP2PPP/R1BQKB1R w KQkq - 0 7";
pub const CHECK_FEN: &str = "6k1/5ppp/8/8/8/8/5PPP/3R2K1 b - - 0 1";
pub const HIGHLIGHT_FEN: &str = "rnbqkbnr/pppp1ppp/8/4p3/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2";

/// The full golden-test set (`tests/golden/<name>.svg`).
pub fn golden_scenarios() -> Vec<(&'static str, &'static str, Options)> {
    vec![
        ("start", START_FEN, Options::default()),
        ("midgame", MIDGAME_FEN, Options::default()),
        (
            "mate",
            CHECK_FEN,
            Options {
                check: Square::from_algebraic("g8"),
                ..Options::default()
            },
        ),
        (
            "flipped",
            START_FEN,
            Options {
                orientation: Color::Black,
                ..Options::default()
            },
        ),
        (
            "highlight",
            HIGHLIGHT_FEN,
            Options {
                highlight: vec![
                    Square::from_algebraic("e2").expect("valid square"),
                    Square::from_algebraic("e4").expect("valid square"),
                ],
                ..Options::default()
            },
        ),
        (
            "no-coords",
            START_FEN,
            Options {
                coordinates: false,
                ..Options::default()
            },
        ),
    ]
}

/// The README gallery set (`assets/gallery/<name>.svg`) — a subset of
/// [`golden_scenarios`], renaming `mate` to `check` (same position, the name
/// that actually matches what the gallery is demonstrating).
pub fn gallery_scenarios() -> Vec<(&'static str, &'static str, Options)> {
    golden_scenarios()
        .into_iter()
        .filter_map(|(name, fen, opts)| match name {
            "no-coords" => None,
            "mate" => Some(("check", fen, opts)),
            other => Some((other, fen, opts)),
        })
        .collect()
}
