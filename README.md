# chess-diagram

Render a chess **position** (a FEN string) to a static, self-contained **SVG**
diagram — inline, no client JS, no Node/Chromium, no native dependencies.
Pure Rust; the default build depends on `thiserror` and nothing else.

> **Status: Phase 1 (in progress)** — model, FEN parse, and the `Renderer`
> seam are in place; the SVG renderer places the Cburnett pieces on the board
> grid and draws file/rank coordinate labels. Highlight/check squares and the
> golden-test harness are still to land (see [PROJECT_PLAN.md](PROJECT_PLAN.md)).

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
make gallery    # render sample boards to target/gallery/*.svg
```

## License

MIT (see [LICENSE](LICENSE)) for the crate itself.

The bundled piece artwork is the **Cburnett** set by Colin M. L. Burnett,
vendored as inline SVG in `src/pieces.rs` and used under its BSD license —
the full notice and attribution ship in
[assets/LICENSE-pieces](assets/LICENSE-pieces).
