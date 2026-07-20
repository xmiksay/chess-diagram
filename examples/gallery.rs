//! Renders the sample positions embedded in `README.md` to
//! `assets/gallery/*.svg`. Run with `make gallery` (or `cargo run --example
//! gallery`), then diff-review the result before committing — the
//! `doc_gallery_up_to_date` test fails if these files drift from the
//! renderer.

use chess_diagram::render_svg;
use std::fs;

#[path = "../tests/common/mod.rs"]
mod fixtures;

fn main() {
    fs::create_dir_all("assets/gallery").expect("create gallery dir");
    for (name, fen, opts) in fixtures::gallery_scenarios() {
        let svg = render_svg(fen, &opts).expect("sample FEN renders");
        let path = format!("assets/gallery/{name}.svg");
        fs::write(&path, svg).expect("write gallery file");
        println!("wrote {path}");
    }
}
