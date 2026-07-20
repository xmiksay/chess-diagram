# chess-diagram

Render a chess **position** (a FEN string) to a static, self-contained **SVG**
diagram — inline, no client JS, no Node/Chromium, no native dependencies.
Pure Rust; the default build depends on `thiserror` and nothing else.

## Usage

```rust
let svg = chess_diagram::render_svg(
    "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
    &Default::default(),
)?;
```

With options:

```rust
use chess_diagram::{render_svg, Color, Options, Square};

let opts = Options {
    orientation: Color::Black,
    highlight: Square::from_algebraic("e2").into_iter()
        .chain(Square::from_algebraic("e4"))
        .collect(),
    ..Options::default()
};
let svg = render_svg("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1", &opts)?;
```

## Gallery

Rendered by `make gallery` (`examples/gallery.rs`) and locked in by the
`doc_gallery_up_to_date` test — if a renderer change makes the committed SVGs
under [`assets/gallery/`](assets/gallery) stale, that test fails and names
the file.

<table>
<tr><td>

`rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1`<br>
`Options::default()`

</td><td><img src="https://raw.githubusercontent.com/xmiksay/chess-diagram/master/assets/gallery/start.svg" width="240" alt="Start position"></td></tr>
<tr><td>

`r1bqk2r/pp2bppp/2n2n2/2pp4/4P3/2NP1N2/PPP2PPP/R1BQKB1R w KQkq - 0 7`<br>
`Options::default()`

</td><td><img src="https://raw.githubusercontent.com/xmiksay/chess-diagram/master/assets/gallery/midgame.svg" width="240" alt="Midgame position"></td></tr>
<tr><td>

`rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1`<br>
`Options { orientation: Color::Black, ..Default::default() }`

</td><td><img src="https://raw.githubusercontent.com/xmiksay/chess-diagram/master/assets/gallery/flipped.svg" width="240" alt="Start position, flipped to Black's viewpoint"></td></tr>
<tr><td>

`rnbqkbnr/pppp1ppp/8/4p3/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2`<br>
`Options { highlight: vec![e2, e4], ..Default::default() }`

</td><td><img src="https://raw.githubusercontent.com/xmiksay/chess-diagram/master/assets/gallery/highlight.svg" width="240" alt="Last-move highlight on e2 and e4"></td></tr>
<tr><td>

`6k1/5ppp/8/8/8/8/5PPP/3R2K1 b - - 0 1`<br>
`Options { check: Square::from_algebraic("g8"), ..Default::default() }`

</td><td><img src="https://raw.githubusercontent.com/xmiksay/chess-diagram/master/assets/gallery/check.svg" width="240" alt="King in check on g8"></td></tr>
</table>

## Annotations

`Options::shapes` draws study-style overlays — circles, arrows (straight or
knight-move elbows), and text badges — on top of the board. `Shape` mirrors
chessground's shape model field-for-field, so a `Shape` payload from a
chessground-based frontend forwards straight through with no translation:

```rust
use chess_diagram::{render_svg, ArrowShape, Options, Shape};

let opts = Options {
    shapes: vec![
        // Circle: brush on `orig`, no `dest`.
        Shape { orig: "e5".into(), dest: None, brush: "red".into(), text: None,
                arrow: ArrowShape::default(), width: None },
        // Arrow: `dest` set — `Auto` picks straight, or an elbow on a knight move.
        Shape { orig: "g1".into(), dest: Some("f3".into()), brush: "green".into(),
                text: None, arrow: ArrowShape::Auto, width: None },
        // Text badge: independent of `dest`, so it can ride along an arrow too.
        Shape { orig: "e4".into(), dest: None, brush: "green".into(),
                text: Some("!".into()), arrow: ArrowShape::default(), width: None },
    ],
    ..Options::default()
};
let svg = render_svg("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", &opts)?;
```

<table>
<tr><td>

`rnbqkbnr/pppp1ppp/8/4p3/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2`<br>
`Options { shapes: vec![circle(e4, green), circle(e5, red), circle(c5, "#8338ec")], ..Default::default() }`

</td><td><img src="https://raw.githubusercontent.com/xmiksay/chess-diagram/master/assets/gallery/circles.svg" width="240" alt="Circle annotations on e4, e5, and c5"></td></tr>
<tr><td>

`rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1`<br>
`Options { shapes: vec![arrow(e2, e4, green), arrow(g1, f3, blue), arrow(d1, h5, red)], ..Default::default() }`

</td><td><img src="https://raw.githubusercontent.com/xmiksay/chess-diagram/master/assets/gallery/arrows.svg" width="240" alt="Straight arrow annotations from e2-e4, g1-f3, and d1-h5"></td></tr>
<tr><td>

`rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1`<br>
`Options { shapes: vec![arrow(g1, f3, green), arrow(b8, c6, blue)], ..Default::default() }`

</td><td><img src="https://raw.githubusercontent.com/xmiksay/chess-diagram/master/assets/gallery/knight-arrow.svg" width="240" alt="Elbow arrow annotations for the knight moves g1-f3 and b8-c6"></td></tr>
<tr><td>

`rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1`<br>
`Options { shapes: vec![arrow(d2, d4, green).width(4), arrow(e2, e4, green).width(7), arrow(c2, c4, green).width(11)], arrow_opacity: 0.55, ..Default::default() }`

</td><td><img src="https://raw.githubusercontent.com/xmiksay/chess-diagram/master/assets/gallery/styled-arrows.svg" width="240" alt="Arrows of increasing shaft width (4, 7, 11) drawn semi-transparently, with arrowheads scaling to match"></td></tr>
<tr><td>

`rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1`<br>
`Options { shapes: vec![text(e4, "!", green), text(e5, "?", red), arrow(g1, f3, blue).text("+3.2")], ..Default::default() }`

</td><td><img src="https://raw.githubusercontent.com/xmiksay/chess-diagram/master/assets/gallery/text-badges.svg" width="240" alt="Text badges '!' on e4 and '?' on e5, plus a '+3.2' badge on the g1-f3 arrow"></td></tr>
</table>

Per-arrow shaft width is set with `Shape::width` (`None` uses the default
~7 SVG units); the arrowhead scales with it, so a thicker shaft gets a
proportionally larger head — the chessground look. `Options::arrow_opacity`
(default `1.0`) makes every arrow translucent as one layer; a transparent
`"#rrggbbaa"` brush works too. `Shape::text` draws a brush-colored badge in
the top-right corner of `orig`, independent of `dest` — a shape can carry
both an arrow and a badge at once.

## Design

The core is a pure function `FEN → String`. No I/O, no async, no asset
loading — those belong to the consumers.

- **Parse layer** (`parse`, `Board`, `Square`) — zero-dep, hand-rolled FEN
  placement parse. It never mentions any output format.
- **Renderer seam** — every output format is one impl of the `Renderer` trait
  (`format()` + `render()`). v1 ships `SvgRenderer`; PNG/typst slot in later
  as additive impls, never a rewrite.
- **Features** — `svg` (default, no deps). Planned opt-ins: `pgn`
  (`shakmaty`), `png` (`resvg`). The default feature set never gains a heavy
  dependency.

## Development

Everything runs through the Makefile (`make help` lists all targets):

```bash
make build      # debug build
make test       # unit + integration + doctests
make lint       # cargo fmt --check + clippy -D warnings
make verify     # lint + test — the hard gate before any push
make gallery    # regenerate the README gallery under assets/gallery/*.svg
```

## License

MIT (see [LICENSE](LICENSE)) for the crate itself.

The bundled piece artwork is the **Cburnett** set by Colin M. L. Burnett,
vendored as inline SVG in `src/pieces.rs` and used under its BSD license —
the full notice and attribution ship in
[assets/LICENSE-pieces](assets/LICENSE-pieces).
