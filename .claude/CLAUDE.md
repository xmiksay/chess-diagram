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
  one impl. v1 ships `SvgRenderer` (`src/render/svg.rs`, hand-emitted SVG
  strings, `#[cfg(feature = "svg")]`). `src/options.rs` holds
  `Options`/`Theme`/`Format` shared by all impls.

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
| 2 | README gallery, docs, publish 0.1 to crates.io | todo |
| 3 | `pgn`/`png` features — only when a real consumer asks | deferred |

## Build & test

Drive everything through the Makefile (`make help` lists all targets):

```bash
make build              # debug build
make check              # fast typecheck of all targets
make test               # all tests (unit + integration + doctest)
make lint               # cargo fmt --check + clippy --all-targets -D warnings
make verify             # lint + test — the hard gate before any push
make gallery            # render sample boards to target/gallery/*.svg
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
