//! End-to-end: FEN in, standalone SVG out, through the public API only.
//! Each sample is also written to `target/test-samples/<name>.svg` for
//! eyeballing; Phase 1 replaces the structural asserts with golden snapshots.

use chess_diagram::{render_svg, Color, Options};
use std::fs;
use std::path::Path;

const SAMPLES: &[(&str, &str)] = &[
    (
        "start",
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
    ),
    (
        "midgame",
        "r1bqk2r/pp2bppp/2n2n2/2pp4/4P3/2NP1N2/PPP2PPP/R1BQKB1R w KQkq - 0 7",
    ),
    ("mate", "6k1/5ppp/8/8/8/8/5PPP/3R2K1 b - - 0 1"),
];

fn assert_valid_svg(name: &str, svg: &str) {
    assert!(svg.starts_with("<svg "), "{name}: not an svg root");
    assert!(svg.ends_with("</svg>"), "{name}: unterminated svg");
    assert!(
        svg.contains(r#"xmlns="http://www.w3.org/2000/svg""#),
        "{name}: missing xmlns (not standalone)"
    );
    assert!(!svg.contains("href"), "{name}: external reference found");
}

#[test]
fn samples_render_to_standalone_svg() {
    let out_dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("../test-samples");
    fs::create_dir_all(&out_dir).expect("create test-samples dir");

    for (name, fen) in SAMPLES {
        let svg = render_svg(fen, &Options::default()).expect(name);
        assert_valid_svg(name, &svg);
        fs::write(out_dir.join(format!("{name}.svg")), &svg).expect("write sample");
    }
}

#[test]
fn flipped_board_renders_and_differs() {
    let (name, fen) = SAMPLES[0];
    let white = render_svg(fen, &Options::default()).expect(name);
    let black = render_svg(
        fen,
        &Options {
            orientation: Color::Black,
            ..Options::default()
        },
    )
    .expect(name);
    assert_valid_svg("flipped", &black);
    assert_ne!(white, black);
}

#[test]
fn invalid_fen_is_an_error_not_a_panic() {
    assert!(render_svg("not a fen", &Options::default()).is_err());
    assert!(render_svg("", &Options::default()).is_err());
}
