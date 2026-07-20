//! FEN → PNG bytes. `#[cfg(feature = "png")]` only: renders the crate's own
//! SVG output ([`render_svg`]) and rasterises it via
//! `resvg`/`usvg`/`tiny-skia` into `Options::size`-square PNG bytes. Never
//! touches the parse layer or the SVG renderer — this is a downstream
//! conversion step, not a `Renderer` impl (`Renderer::render` returns a
//! `String`, not bytes).

use std::sync::Arc;

use resvg::tiny_skia::{Pixmap, Transform};
use resvg::usvg::{fontdb, Options as UsvgOptions, Tree};
use thiserror::Error;

use crate::{render_svg, FenError, Options};

/// Errors from rasterising a chess diagram to PNG. Kept separate from
/// [`FenError`] so the default (SVG-only) build's error
/// surface is unaffected by this feature.
#[derive(Debug, Error)]
pub enum PngError {
    /// `fen` failed to parse (the same error [`render_svg`] returns).
    #[error(transparent)]
    Fen(#[from] FenError),
    /// The freshly rendered SVG failed to parse back into a `resvg`/`usvg`
    /// tree. Should not happen for SVG produced by this crate's own
    /// `SvgRenderer`.
    #[error("failed to parse the rendered SVG for rasterisation: {0}")]
    Svg(#[from] resvg::usvg::Error),
    /// `Options::size` is `0`, so no pixmap can be allocated.
    #[error("rendered size {0} is not a valid pixmap size")]
    InvalidSize(u32),
}

/// Renders `fen` to a `Options::size` × `Options::size` PNG image.
///
/// Internally renders to SVG via [`render_svg`] and
/// rasterises with `resvg`. Coordinate labels and text badges use
/// `sans-serif`; resolving that requires a font database, so this loads the
/// host's system fonts via `fontdb` — glyph shapes (and thus exact pixel
/// output) are environment-dependent. Treat the PNG bytes as opaque and test
/// structurally (dimensions, non-empty, decodes) rather than byte-for-byte.
///
/// # Example
///
/// ```
/// use chess_diagram::{png::render_png, Options};
///
/// let bytes = render_png("8/8/8/8/8/8/8/4K3", &Options::default())?;
/// assert!(bytes.starts_with(&[0x89, b'P', b'N', b'G']));
/// # Ok::<(), chess_diagram::png::PngError>(())
/// ```
pub fn render_png(fen: &str, opts: &Options) -> Result<Vec<u8>, PngError> {
    // Checked up front: a size-0 SVG is itself invalid (no width/height/viewBox
    // usvg would accept), so this would otherwise surface as a confusing
    // `PngError::Svg` instead of naming the actual problem.
    if opts.size == 0 {
        return Err(PngError::InvalidSize(0));
    }

    let svg = render_svg(fen, opts)?;

    let mut fonts = fontdb::Database::new();
    fonts.load_system_fonts();
    let usvg_opts = UsvgOptions {
        fontdb: Arc::new(fonts),
        ..Default::default()
    };
    let tree = Tree::from_str(&svg, &usvg_opts)?;

    let mut pixmap = Pixmap::new(opts.size, opts.size).ok_or(PngError::InvalidSize(opts.size))?;
    resvg::render(&tree, Transform::identity(), &mut pixmap.as_mut());

    Ok(pixmap
        .encode_png()
        .expect("encoding a freshly-allocated in-memory pixmap to PNG cannot fail"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decoded_size(png: &[u8]) -> (u32, u32) {
        let decoder = png::Decoder::new(std::io::Cursor::new(png));
        let reader = decoder.read_info().expect("valid PNG");
        let info = reader.info();
        (info.width, info.height)
    }

    #[test]
    fn renders_default_size_png() {
        let bytes = render_png(
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            &Options::default(),
        )
        .expect("valid FEN");
        assert!(bytes.starts_with(&[0x89, b'P', b'N', b'G']));
        assert_eq!(decoded_size(&bytes), (360, 360));
    }

    #[test]
    fn honors_custom_size() {
        let opts = Options {
            size: 512,
            ..Default::default()
        };
        let bytes = render_png("8/8/8/8/8/8/8/4K3", &opts).expect("valid FEN");
        assert_eq!(decoded_size(&bytes), (512, 512));
    }

    #[test]
    fn invalid_fen_is_rejected_before_rasterising() {
        let err = render_png("not-a-fen", &Options::default()).unwrap_err();
        assert!(matches!(err, PngError::Fen(_)));
    }

    #[test]
    fn zero_size_is_rejected() {
        let opts = Options {
            size: 0,
            ..Default::default()
        };
        let err = render_png("8/8/8/8/8/8/8/4K3", &opts).unwrap_err();
        assert!(matches!(err, PngError::InvalidSize(0)));
    }
}
