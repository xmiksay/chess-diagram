# chess-diagram

Single-crate Rust library that renders a chess position (FEN) to a static,
self-contained SVG diagram. Pure Rust — the default build depends on
`thiserror` and nothing else. No I/O, no async: the core is a pure function
`FEN → String`. Full design rationale and delivery plan live in
[../PROJECT_PLAN.md](../PROJECT_PLAN.md).

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
  on the other — long leg first, short leg last, the lichess convention).
  `src/options.rs` holds `Options`/`Theme`/`Format` shared by
  all impls; `src/annotation.rs` holds the `Shape`/`ArrowShape` annotation
  model (mirrors chessground's `Shape` — not part of `board.rs`, since
  annotations are a rendering concern).

Extension seams:
- **New output format** = new `Format` variant (the enum is
  `#[non_exhaustive]`) + one new `Renderer` impl under `src/render/`. Never a
  parse-layer change.
- **Heavier capability** = opt-in Cargo feature: `pgn` (gates `shakmaty`),
  `png` (gates `resvg`). **The default feature set never gains a heavy
  dependency.**

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
| 3 | `pgn`/`png` features — only when a real consumer asks | deferred |

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

- Errors via `thiserror`; `FenError` is the only error type. **No panics on
  any input** — no `unwrap`/`expect`/indexing on reachable paths (tests and
  examples excepted).
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
