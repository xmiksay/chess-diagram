//! End-to-end: FEN in, standalone SVG out, through the public API only.
//!
//! Each fixture also writes an eyeball copy to `target/test-samples/<name>.svg`.
//! The golden fixtures under `tests/golden/<name>.svg` lock the exact
//! rendered bytes so a renderer regression fails loudly instead of drifting
//! silently. Regenerate after an intentional rendering change with
//! `UPDATE_GOLDEN=1 cargo test` (or `make golden-update`).
//!
//! `doc_gallery_up_to_date` locks the README gallery under `assets/gallery/`
//! the same way — regenerate with `make gallery` after a rendering change.

use chess_diagram::{render_svg, Color, Options};
use std::fs;
use std::path::{Path, PathBuf};

#[path = "common/mod.rs"]
mod fixtures;
use fixtures::START_FEN;

fn assert_valid_svg(name: &str, svg: &str) {
    assert!(svg.starts_with("<svg "), "{name}: not an svg root");
    assert!(svg.ends_with("</svg>"), "{name}: unterminated svg");
    assert!(
        svg.contains(r#"xmlns="http://www.w3.org/2000/svg""#),
        "{name}: missing xmlns (not standalone)"
    );
    assert!(!svg.contains("href"), "{name}: external reference found");
}

fn golden_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(format!("{name}.svg"))
}

/// Compares `actual` byte-for-byte against the committed fixture for `name`.
/// With `UPDATE_GOLDEN=1` set, rewrites the fixture instead of comparing —
/// the accepted flow for landing an intentional rendering change.
fn assert_matches_golden(name: &str, actual: &str) {
    let path = golden_path(name);
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        fs::write(&path, actual)
            .unwrap_or_else(|e| panic!("failed to write golden {}: {e}", path.display()));
        return;
    }
    let expected = fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "failed to read golden {} ({e}); run `UPDATE_GOLDEN=1 cargo test` to create it",
            path.display()
        )
    });
    if actual != expected {
        panic!(
            "golden mismatch for `{name}` ({}):\n{}\nrun `UPDATE_GOLDEN=1 cargo test` (or `make golden-update`) to accept this output",
            path.display(),
            diff_hint(&expected, actual),
        );
    }
}

/// Short hint for a golden mismatch: byte offset and a snippet either side of
/// the first difference, or a length note if one is a prefix of the other.
fn diff_hint(expected: &str, actual: &str) -> String {
    let mismatch = expected
        .as_bytes()
        .iter()
        .zip(actual.as_bytes())
        .position(|(a, b)| a != b);
    let Some(at) = mismatch else {
        return format!(
            "expected {} bytes, actual {} bytes (one is a prefix of the other)",
            expected.len(),
            actual.len()
        );
    };
    const CTX: usize = 30;
    format!(
        "first difference at byte {at}:\n  expected: ...{}...\n  actual:   ...{}...",
        snippet(expected, at, CTX),
        snippet(actual, at, CTX),
    )
}

/// `ctx` bytes of `s` on either side of `at`, clamped to the string bounds.
fn snippet(s: &str, at: usize, ctx: usize) -> &str {
    let start = at.saturating_sub(ctx);
    let end = (at + ctx).min(s.len());
    &s[start..end]
}

#[test]
fn golden_snapshots_match_committed_fixtures() {
    let out_dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("../test-samples");
    fs::create_dir_all(&out_dir).expect("create test-samples dir");

    for (name, fen, opts) in fixtures::golden_scenarios() {
        let svg = render_svg(fen, &opts).expect(name);
        assert_valid_svg(name, &svg);
        fs::write(out_dir.join(format!("{name}.svg")), &svg).expect("write sample");
        assert_matches_golden(name, &svg);
    }
}

fn gallery_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("assets/gallery")
        .join(format!("{name}.svg"))
}

/// Keeps `assets/gallery/*.svg` (embedded in `README.md`) in sync with the
/// renderer: fails and names the stale file instead of letting the README
/// drift silently. Regenerate with `make gallery`.
#[test]
fn doc_gallery_up_to_date() {
    for (name, fen, opts) in fixtures::gallery_scenarios() {
        let svg = render_svg(fen, &opts).expect(name);
        let path = gallery_path(name);
        let committed = fs::read_to_string(&path).unwrap_or_else(|e| {
            panic!(
                "failed to read {} ({e}); run `make gallery` to create it",
                path.display()
            )
        });
        assert!(
            svg == committed,
            "assets/gallery/{name}.svg is stale — run `make gallery` and commit the diff\n{}",
            diff_hint(&committed, &svg),
        );
    }
}

#[test]
fn flipped_board_renders_and_differs() {
    let white = render_svg(START_FEN, &Options::default()).expect("start");
    let black = render_svg(
        START_FEN,
        &Options {
            orientation: Color::Black,
            ..Options::default()
        },
    )
    .expect("start");
    assert_ne!(white, black);
}

#[test]
fn invalid_fen_is_an_error_not_a_panic() {
    assert!(render_svg("not a fen", &Options::default()).is_err());
    assert!(render_svg("", &Options::default()).is_err());
}
