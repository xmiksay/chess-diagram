//! Renders the sample positions to `target/gallery/*.svg`.
//! Run with `make gallery` (or `cargo run --example gallery`).

use chess_diagram::{render_svg, Color, Options};
use std::fs;

fn main() {
    let samples = [
        (
            "start",
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            Options::default(),
        ),
        (
            "midgame",
            "r1bqk2r/pp2bppp/2n2n2/2pp4/4P3/2NP1N2/PPP2PPP/R1BQKB1R w KQkq - 0 7",
            Options::default(),
        ),
        (
            "flipped",
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            Options {
                orientation: Color::Black,
                ..Options::default()
            },
        ),
    ];

    fs::create_dir_all("target/gallery").expect("create gallery dir");
    for (name, fen, opts) in samples {
        let svg = render_svg(fen, &opts).expect("sample FEN renders");
        let path = format!("target/gallery/{name}.svg");
        fs::write(&path, svg).expect("write gallery file");
        println!("wrote {path}");
    }
}
