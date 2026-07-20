# chess-diagram

Single-crate Rust library that renders a chess position (FEN) to a static,
self-contained SVG diagram, with opt-in PNG rasterisation. Pure Rust — the
default build depends on `thiserror` and nothing else. No I/O, no async: the
core is a pure function `FEN → String`. Full design rationale and delivery
plan live in [../PROJECT_PLAN.md](../PROJECT_PLAN.md).

## Architecture overview

Two layers behind one seam:

- **Parse layer** — `src/fen.rs` (hand-rolled FEN placement parse, the crate's
  only fallible step → `FenError`) produces the `src/board.rs` model
  (`Board`/`Piece`/`Color`/`Role`/`Square`; `a1 = 0`). **Hard rule:** no
  format name and no SVG string ever appears in this layer.
- **Renderer seam** — `src/render/mod.rs` defines `Renderer { format(),
  render() }` (modeled on mdcast's `Backend` trait); every output format is
  one impl. v1 ships `SvgRenderer` (`src/render/svg/`, hand-emitted SVG
  strings, `#[cfg(feature = "svg")]`), split into `mod.rs` (orchestration +
  shared geometry incl. `square_center`), `grid.rs` (base squares +
  highlight/check overlays), `coordinates.rs` (file/rank labels), `shapes.rs`
  (`Options::shapes` single-square annotations — circles today, drawn between
  the overlays and the pieces), and `arrows.rs` (arrows for `Shape::dest`,
  drawn above the pieces: a `<marker>`-per-brush-color `<defs>` block right
  after the `<svg>` header — a 4×4 `markerUnits="strokeWidth"` arrowhead
  (chessground's proportions, so the head is ~4× the shaft and auto-scales
  with each arrow's stroke width) — then a `<line>` per straight arrow or a
  `<polyline>` per elbowed one, both with `marker-end`. Shaft width is
  `Shape::width` (`None` → `DEFAULT_ARROW_WIDTH` ≈ 7). `Options::arrow_opacity`
  (< 1.0) wraps each arrow in a `<g opacity>` so shaft + head composite as one
  translucent layer without double-darkening. `Shape::arrow`
  resolves to straight or elbow: `Straight`/`Elbow` force the shaft shape,
  `Auto` picks elbow for a knight-move `orig`/`dest` offset (file/rank deltas
  of `(1, 2)`/`(2, 1)`) and straight otherwise. The elbow bends at a corner
  sharing `dest`'s coordinate on the axis with the larger delta and `orig`'s
  on the other — long leg first, short leg last, the lichess convention), and
  `text.rs` (`Shape::text` per-square badges, drawn last — above the arrows —
  as a `<text>` in the top-right corner of `orig`, following
  `write_coordinates`'s font-family/font-size/inset pattern; independent of
  `Shape::dest`, so one shape can carry both an arrow and a badge; badge
  content is XML-escaped so arbitrary text can't corrupt the markup).
  `src/options.rs` holds `Options`/`Theme`/`Format` shared by
  all impls; `src/annotation.rs` holds the `Shape`/`ArrowShape` annotation
  model (mirrors chessground's `Shape` — not part of `board.rs`, since
  annotations are a rendering concern).
- **PGN layer** — `src/pgn.rs` (`#[cfg(feature = "pgn")]`, opt-in): `pub fn
  board_at(pgn: &str, ply: usize) -> Result<Board, PgnError>` walks a PGN
  mainline via `shakmaty` (SAN disambiguation, castling, en passant,
  promotion all delegated to it) and converts `shakmaty`'s `Board`/`Color`/
  `Role`/`Piece` into this crate's model. `mainline_sans` tokenizes the
  movetext (drops header-tag lines whole since a quoted value may contain
  whitespace; tracks brace/paren depth across words so multi-word comments
  and variations are skipped in full; stops at a result token). Errors are
  `PgnError`, a type separate from `FenError` — this module never touches
  the parse layer or its error contract.
- **PNG layer** — `src/png.rs` (`#[cfg(feature = "png")]`, opt-in, implies
  `svg`): `pub fn render_png(fen: &str, opts: &Options) -> Result<Vec<u8>,
  PngError>` calls `render_svg` then rasterises the resulting SVG string with
  `resvg`/`usvg`/`tiny-skia` into an `Options::size`-square `Pixmap`, encoded
  to PNG bytes. Not a `Renderer` impl — `Renderer::render` returns a `String`,
  this returns bytes, so it's a downstream conversion step over `SvgRenderer`'s
  own output instead. Coordinate labels and text badges use `sans-serif`, so
  `usvg::Options::fontdb` is populated via `fontdb::Database::load_system_fonts`
  — glyph shapes (and thus exact pixel output) are environment-dependent, so
  tests check structure (`size × size`, decodes as PNG) rather than golden
  bytes. `PngError` is a third error type, separate from `FenError`/`PgnError`.

Extension seams:
- **New output format** = new `Format` variant (the enum is
  `#[non_exhaustive]`) + one new `Renderer` impl under `src/render/`. Never a
  parse-layer change.
- **Heavier capability** = opt-in Cargo feature: `pgn` (gates `shakmaty`) and
  `png` (gates `resvg`, implies `svg`), both landed. **The default feature set
  never gains a heavy dependency.**

## Status

| Phase | Content | Status |
|---|---|---|
| 0 | Model + FEN parse + `Renderer` seam + stub `SvgRenderer` (grid only) | done |
| 1 | Full SVG: Cburnett pieces (`src/pieces.rs`) placed on the grid | done |
| 1 | Coordinate labels (`Options::coordinates`) | done |
| 1 | Highlight/check squares (`Options::highlight`/`check`) | done |
| 1 | Golden-test harness | done |
| 2 | README gallery (`assets/gallery/`) + freshness test | done |
| 2 | Rustdoc pass: crate docs, doc examples, `#![warn(missing_docs)]` | done |
| 2 | Package metadata + contents + license hygiene (`Cargo.toml` `exclude`, README image URLs) | done |
| 2 | Release workflow: tag-triggered, crates.io Trusted Publishing (OIDC) | done |
| 2 | Publish 0.1 to crates.io | done |
| 3 | Split `src/render/svg.rs` into `src/render/svg/` submodule (annotations prereq) | done |
| 3 | Annotation model (`src/annotation.rs`: `Shape`/`ArrowShape`) + `Options::shapes` + brush palette (`Theme::{green,red,blue,yellow}`, `Theme::resolve_brush`) + circle rendering | done |
| 3 | Straight arrows (`Shape::dest` + `Straight`/non-knight `Auto`, `src/render/svg/arrows.rs`) rendered above the pieces | done |
| 3 | Knight/elbow arrows (`Shape::dest` + `Elbow`/knight-move `Auto`, `src/render/svg/arrows.rs`) — bent shaft, same marker/trim conventions | done |
| 3 | Arrow sizing: chessground-proportioned arrowhead (was 10× oversized), per-shape `Shape::width`, `Options::arrow_opacity` group transparency | done |
| 3 | Per-square text badges (`Shape::text`, `src/render/svg/text.rs`) rendered above the arrows, XML-escaped | done |
| 3 | `pgn` feature: `pgn::board_at` (FEN-at-move-N via `shakmaty`), `PgnError`, feature-matrix CI job | done |
| 3 | `png` feature: `png::render_png` (SVG → PNG bytes via `resvg`, `src/png.rs`), `PngError`, system-fontdb text | done |
| 3 | Theme presets: `Theme::brown()`/`blue()`/`green()` (plain constructors, `Theme` stays hand-constructible) | done |

## Build & test

Drive everything through the Makefile (`make help` lists all targets):

```bash
make build              # debug build
make check              # fast typecheck of all targets
make test               # all tests (unit + integration + doctest)
make lint               # cargo fmt --check + clippy --all-targets -D warnings
make verify             # lint + test — the hard gate before any push
make gallery            # regenerate the README gallery under assets/gallery/*.svg
make golden-update      # rewrite tests/golden/*.svg from current renderer output
make package            # cargo package --allow-dirty
```

`CARGO_BUILD_JOBS` is pinned to 4 in the Makefile (override per invocation).

Integration tests (`tests/integration.rs`) write one sample SVG per fixture to
`target/test-samples/<name>.svg` for eyeballing, and byte-compare each render
against a committed fixture under `tests/golden/<name>.svg` (`start`,
`midgame`, `mate`, `flipped`, `highlight`, `no-coords`). A mismatch panics
with the fixture path and a snippet around the first differing byte. After an
*intentional* rendering change, regenerate the fixtures with
`UPDATE_GOLDEN=1 cargo test` (or `make golden-update`), then diff-review the
changed `tests/golden/*.svg` files before committing — a stale-looking diff
there is the signal a regression slipped in unreviewed.

The README gallery reuses the same sample set (`tests/common/mod.rs`, shared
by `tests/integration.rs` and `examples/gallery.rs` via `#[path]`) rendered to
`assets/gallery/<name>.svg` and embedded in `README.md`. The
`doc_gallery_up_to_date` test byte-compares each render against the committed
file and fails naming the stale one; regenerate with `make gallery`, then
diff-review before committing.

CI (`.github/workflows/ci.yml`) runs a `feature-matrix` job alongside the
main `test` job: `cargo check --no-default-features --lib` (proves the bare
parse layer still compiles `thiserror`-only with every optional feature off)
and `cargo test --all-features --all-targets` (exercises `pgn` and `png`
together, and any future opt-in feature). Run the `png` tests locally with
`cargo test --features png`.

## Release procedure

Publishing to crates.io is tag-triggered (`.github/workflows/release.yml`),
authenticating via crates.io **Trusted Publishing (OIDC)** — no
`CARGO_REGISTRY_TOKEN` secret exists or is needed in this repo.

To cut a release:

1. Bump the `version` in `Cargo.toml`.
2. `make verify` locally, commit, merge to `master`.
3. Tag and push: `git tag v<version> && git push origin v<version>` — the
   tag must match `Cargo.toml`'s version exactly (the workflow fails the
   release otherwise).
4. The `release.yml` workflow re-runs the full verify gate (fmt, clippy,
   tests) then publishes via `cargo publish` under OIDC.

One-time setup (already documented in the workflow header comment): the
crates.io trusted publisher for `xmiksay/chess-diagram` must be configured
with workflow filename `release.yml` before the first tag push.

**Note:** `0.1.0` was published manually (`cargo publish`) — the trusted
publisher for `xmiksay/chess-diagram` was not registered before the `v0.1.0`
tag push, so `release.yml`'s OIDC token exchange failed with "No Trusted
Publishing config found for repository". The crate now exists on crates.io,
so the trusted publisher can be added from its Settings page (step 2 in the
workflow header) before the next tag push, restoring the tag-triggered flow
for `0.2.0`+.

## Conventions

- Errors via `thiserror`; `FenError` is the only error type of the default
  build (`pgn`/`png` each add their own `PgnError`/`PngError` behind their
  feature gate — never grow `FenError` to cover them). **No panics on any
  input** — no `unwrap`/`expect`/indexing on reachable paths (tests and
  examples excepted; a proven-unreachable panic path may use `.expect("…")`
  naming the invariant, e.g. `png::render_png`'s PNG encode of a
  freshly-allocated pixmap).
- Comments only where the *why* is non-obvious from the code.
- Tests: unit tests in `#[cfg(test)] mod tests` at the end of each file;
  end-to-end tests in `tests/integration.rs`. Every change ships with tests.
- Keep files small — under 400 LoC; split a module before it grows past that.
- DRY and KISS: no premature abstraction; the `Renderer` trait is the one
  deliberate seam.
- SVG output must stay self-contained: no external refs, no JS, opens
  standalone in a browser.
- When adding functionality, refresh the docs in the same change: this file
  (Status table included), `README.md`, and `Cargo.toml`
  (description/keywords).
- **Hard gate: never push until `make verify` passes** (fmt-check + clippy
  `-D warnings` + full test suite). A red `verify` means the branch stays
  local.
