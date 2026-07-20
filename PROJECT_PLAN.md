# chess-diagram — Project Plan

> **Crate name:** `chess-diagram` (verified free on crates.io as of writing;
> `boardgen`, `fen-diagram`, `chessfig`, `position-diagram` are also free).
> `chess-render` is **taken** (an embeddable UCI GUI — unrelated).
>
> The name is deliberately **format-neutral**. `mermaid-svg` bakes `svg` into
> its name, which reads wrong the moment you consider any other output; this
> crate does only SVG in v1, but is named so PNG/typst output can be added later
> without a rename.

## 1. Goal

A focused Rust crate that takes a chess **position** (a FEN string) and renders
it to a static **SVG** diagram — inline, self-contained, no client JS.

The crate is deliberately small and boring: one clean thing done well, reusable
across every chess project in the workspace. SVG is the only v1 format; the
interface is built so more formats are additive, not a rewrite.

## 2. Guiding principle

> **Interface like the typst renderer; footprint like `mermaid-svg`.**

Two independent design axes, pulled from two in-house crates:

- **Interface** — model the renderer *seam* on mdcast's `Backend` trait
  (`src/backends/typst/mod.rs`): every output format is **one impl** behind a
  `format() + render()` trait; adding a format is an additive change at the
  seam, never a rewrite of the core. v1 has a single impl (SVG); the seam exists
  so PNG/typst slot in later.
- **Footprint** — match `mermaid-svg`: **pure Rust, `thiserror` only**, no
  Node/Chromium, no native deps, and crucially **no typst compiler and no
  `shakmaty` in the core**. SVG is hand-emitted as strings (exactly how
  `mermaid-svg` builds its `svg/` output); FEN parsing is ~30 trivial lines.
  Anything heavier (PGN move-walking, PNG rasterisation) is an **opt-in
  feature**, so the default build stays tiny.

The core is a pure function `FEN → String`. No I/O, no async, no asset provider
— those belong to the *consumers* (site, mdcast, ChessBase), not here.

## 3. Locked decisions

| Decision | Choice | Rationale |
|---|---|---|
| v1 output | **SVG only** | The one format every consumer needs. Named format-neutrally so PNG/typst are additive later, not a rename. |
| Rendering technique | **Hand-emitted SVG strings** (like `mermaid-svg`) | Keeps deps at zero. A typst-compiled board would pull in `typst`/`typst-pdf`/`typst-as-lib` — tens of heavy crates — for output the consumer (mdcast) already knows how to place. Wrong layer. |
| Chess logic in core | **None** — hand-rolled FEN piece-placement parse only | Rendering a position needs only the placement field (ranks split on `/`, digits = gaps, letters = pieces). `shakmaty` would be a large dep for a 30-line parse. |
| PGN → position | **Deferred to an optional `pgn` feature** (or the caller) | SAN application (disambiguation, castling, en passant, promotion) is real work; when needed it gates `shakmaty` behind a feature so the FEN-only core stays dep-free. The site already knows the move index. |
| Renderer interface | **`Renderer` trait: `format()` + `render()`** | Direct analog of mdcast's `Backend { target(); render_to_bytes() }`, minus async/assets. The one v1 impl is `SvgRenderer`; the trait is the extensibility seam. |
| Convenience API | **Free functions `render_svg` / `parse`** | Mirrors `mermaid-svg`'s `render` / `render_with` / `parse` ergonomics for the one-liner case. |
| Piece artwork | **Cburnett set, BSD variant, vendored as `&str` consts** | Public, widely used, license-clean; no runtime asset loading (no `rust-embed`) — pieces are just string constants, same spirit as `mermaid-svg`'s inline markup. |
| Error handling | **`thiserror`, one `FenError`** | Only fallible step is FEN parse. No panics on any input. |

## 4. Scope

**In scope (v1):**

- FEN parse → `Board` model (placement + side-to-move + orientation).
- `SvgRenderer`: board, squares, coordinates, orientation, last-move highlight, check highlight.
- `Renderer` trait + free-function convenience wrappers.
- Golden-snapshot tests; a small `examples/` gallery.

**Out of scope (v1), addable later at the existing seam:**

- PGN parsing / move-walking (`pgn` feature).
- PNG/raster output (`png` feature → `resvg`).
- Any non-SVG renderer.
- Arrows, square annotations, custom themes/piece-sets beyond a default.
- Animation / move sequences.

## 5. Public API — the load-bearing spec

The interface is the deliverable users pin against; everything else is
replaceable behind it.

```rust
// ── parse layer (pure, zero-dep) — mirrors mermaid_svg::parse ──────────────
pub fn parse(fen: &str) -> Result<Board, FenError>;

pub struct Board { /* [Option<Piece>; 64], side_to_move, … */ }
pub struct Piece { pub color: Color, pub role: Role }
pub enum Color { White, Black }
pub enum Role  { Pawn, Knight, Bishop, Rook, Queen, King }
pub struct Square(u8);          // 0..64, a1 = 0
impl Square { pub fn from_algebraic(s: &str) -> Option<Square>; }

// ── options ────────────────────────────────────────────────────────────────
#[derive(Default)]
pub struct Options {
    pub orientation: Color,     // board POV (default White)
    pub highlight: Vec<Square>, // e.g. last-move from/to
    pub check: Option<Square>,  // king-in-check square
    pub coordinates: bool,
    pub size: u32,              // SVG px edge
    pub theme: Theme,           // light/dark square colors
}

// ── renderer interface — modeled on mdcast's `Backend` trait ────────────────
pub enum Format { Svg }         // one v1 variant; Png/Typst added later

pub trait Renderer {
    fn format(&self) -> Format;
    fn render(&self, board: &Board, opts: &Options) -> String;
}
pub struct SvgRenderer;         // impl Renderer — the only v1 impl

// ── convenience free fns — mirror mermaid_svg::render / render_with ─────────
pub fn render_svg(fen: &str, opts: &Options) -> Result<String, FenError>; // parse + SvgRenderer

// one-liner, like `mermaid_svg::render("…")`
// let svg = chess_diagram::render_svg("rnbqkbnr/…", &Default::default())?;
```

**Rule (borrowed from mdcast):** no format name and no SVG string ever leaks
into the `Board`/parse layer. If it does, the seam is wrong.

## 6. Format → renderer map

| Format | Renderer | Extra deps | Notes |
|---|---|---|---|
| `svg` | `SvgRenderer` | **none** | Hand-emitted SVG + Cburnett `&str` piece consts. Default feature. |
| `png` *(future)* | rasterise the SVG | `resvg` (feature `png`) | `render_png(...) -> Vec<u8>`; opt-in, returns bytes. |
| `typst` *(future)* | emit typst markup | none (string) | only if mdcast wants a native-typst board; same trait. |

## 7. Project structure

Single crate. The one v1 renderer is a module; the only optional weight is
behind features, so the default build is `thiserror` and nothing else.

```
chess-diagram/
├─ Cargo.toml                 # features: svg (default), pgn, png
├─ src/
│  ├─ lib.rs                  # pub API: parse, Board, Renderer, render_svg, re-exports
│  ├─ fen.rs                  # FEN placement parse → Board  (FenError)
│  ├─ board.rs                # Board / Piece / Color / Role / Square model
│  ├─ options.rs             # Options, Theme, Format
│  ├─ render/
│  │  ├─ mod.rs               # Renderer trait + dispatch
│  │  └─ svg/                 # #[cfg(feature = "svg")] SvgRenderer
│  │     ├─ mod.rs            # SvgRenderer + render() orchestration, shared geometry
│  │     ├─ grid.rs           # base squares + highlight/check overlay rects
│  │     └─ coordinates.rs    # file/rank coordinate labels
│  ├─ pieces.rs               # Cburnett BSD piece SVG path consts (feature = "svg")
│  └─ pgn.rs                  # #[cfg(feature = "pgn")] FEN-at-move-N (gates shakmaty)
├─ examples/gallery.rs        # render sample FENs to files (doc gallery)
├─ tests/golden/              # snapshot fixtures (start pos, mid-game, mate, flipped)
├─ assets/LICENSE-pieces      # Cburnett BSD notice
├─ README.md
└─ Makefile                   # build / test / lint / fmt / gallery
```

```toml
# Cargo.toml — feature sketch
[features]
default = ["svg"]
svg   = []                 # hand-emitted SVG + embedded piece consts (no deps)
pgn   = ["dep:shakmaty"]   # walk a PGN to move N → FEN; opt-in heavy dep
png   = ["svg", "dep:resvg"] # rasterise SVG → PNG bytes; opt-in

[dependencies]
thiserror = "2"
shakmaty  = { version = "0.27", optional = true }
resvg     = { version = "0.44", optional = true }
```

> Stays a single crate. The load-bearing boundary is the `Renderer` trait, not
> the crate count — split to a workspace only if a feature grows native deps
> that pollute builds for people who don't enable it.

## 8. Delivery plan (ordered by dependency, not calendar)

### Phase 0 — Skeleton + model
`Board`/`Piece`/`Square` model, `fen.rs` parse with `FenError`, the `Renderer`
trait + `Format`, and `render_svg` wired to a stub `SvgRenderer`.
**Done when:** `parse(fen)` round-trips a handful of positions and `render_svg`
dispatches through the trait (emitting placeholder output).

### Phase 1 — SVG renderer
Board grid + square colours + coordinates + orientation, then Cburnett pieces
placed from `pieces.rs`. Then highlight (last-move) and check squares. Lock the
golden-test harness here so later refactors can't silently regress.
**Done when:** start position, a mid-game FEN, and a flipped board render to
self-contained SVG (opens standalone, no external refs, no JS), byte-stable
under golden test.

### Phase 2 — Polish + publish
`README` with a rendered gallery (like `mermaid-svg`'s), doc examples,
`Makefile`, CI (fmt + clippy + test), MIT-or-Apache license + Cburnett BSD
notice, publish `0.1` to crates.io.
**Done when:** `cargo publish --dry-run` is clean and docs.rs renders.

### Phase 3 — Optional features (deferred, on demand)
`pgn` (walk to move N via `shakmaty`) and `png` (`resvg`) — each an additive
feature-gated module, triggered by a real consumer asking, not pre-built.
**Done when:** a consumer that needs one enables the feature; default builds are
byte-identical and dep-identical to before.

## 9. Sequencing logic

Prove the seam cheaply (0) → do the artwork-heavy SVG format against a locked
harness (1) → make it publishable (2) → grow features only when pulled (3).
Every later item lands as an additive change behind the `Renderer` trait or a
Cargo feature, never a teardown.

## 10. Future evolution — with explicit triggers

| Future move | Trigger (not "nice to have") | Scope when taken |
|---|---|---|
| `pgn` feature | A consumer needs FEN-at-move-N and can't compute it (the site can, ChessBase already parses PGN) | One `pgn.rs` module gating `shakmaty`; core untouched. |
| `png` feature | A consumer needs raster (thumbnail, non-SVG channel) | `render_png → Vec<u8>` via `resvg`, gated `png`. |
| `typst` output | mdcast wants a native-typst board instead of an embedded SVG | New `Format::Typst` impl of the **same trait**; string output, no new dep. |
| Arrows / annotations | Study/analysis rendering needs them (ChessBase) | Additive `Options` fields + SVG emission. |
| Custom piece sets / themes | Branding parity with a consumer's design | `Theme`/piece-set indirection already in `Options`; add variants. |

**Hard rule:** the default feature set never gains a heavy dependency. Every
engine-grade dep (typst, shakmaty, resvg) lives behind an opt-in feature. That
is the whole point of "footprint like `mermaid-svg`."

## 11. Downstream consumers (why this earns its place)

- **personal-site** `#66` (mdcast export bridge) — consumes `features=["svg"]`
  to render `<fen>`/`<pgn>` statically into PDF/slides (no client JS).
- **personal-site** `#58` — the same server-side SVG lets the **live** site drop
  its `chess-viewer.js` + chessboard.js + **jQuery-from-CDN** dependency for
  static boards.
- **Orechov** chess-club site — same `<fen>`/`<pgn>` directives, same SVG need.
- **ChessBase MCP** — SVG boards for studies/reports; later `pgn`/arrows.

Three-plus real consumers already exist, so this clears the workspace
"DRY-only-after-the-second-occurrence" bar — it is reuse, not speculation.

## 12. Definition of "solid start" (v1)

`Board` + `fen.rs` parse + `Renderer` trait + `SvgRenderer` + `Options`/`Theme`
+ `render_svg` wrapper + golden tests + a rendered README gallery — **pure Rust,
`thiserror`-only default build**, with `pgn`/`png` reachable as additive
feature-gated modules. A few hundred lines of string assembly behind one trait,
mirroring `mermaid-svg`'s footprint and mdcast's renderer seam.
