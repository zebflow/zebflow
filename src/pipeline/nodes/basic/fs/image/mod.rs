//! `fs.image.*` — content operations on raster images: `fs.image.thumbnail`,
//! `fs.image.chromakey`. The decoder with its decompression-bomb limits is
//! shared here.

use std::io::Cursor;

use image::DynamicImage;

use crate::pipeline::PipelineError;

pub mod chromakey;
pub mod thumbnail;

/// Max decompressed image side (px) — prevents decompression bombs.
const MAX_DIM: u32 = 16_000;
/// Max memory allocated for image decode (128 MB).
const MAX_ALLOC: u64 = 128 * 1024 * 1024;

/// Load an image with hard dimension + allocation limits (decompression bomb protection).
///
/// Strategy: read only the image header first via `into_dimensions()` (fast, no full decode),
/// reject if too large, then do the full decode. This prevents PNG/WebP bombs where a tiny
/// file expands to gigabytes in memory.
pub fn load_with_limits(bytes: &[u8]) -> Result<DynamicImage, PipelineError> {
    // Step 1: read header only — check dimensions before decoding.
    let (w, h) = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| PipelineError::new("FS_IMAGE_DECODE", format!("format detection: {e}")))?
        .into_dimensions()
        .map_err(|e| PipelineError::new("FS_IMAGE_DECODE", format!("image header read: {e}")))?;

    if w > MAX_DIM || h > MAX_DIM {
        return Err(PipelineError::new(
            "FS_IMAGE_DECODE",
            format!("image dimensions {w}x{h} exceed maximum {MAX_DIM}x{MAX_DIM}"),
        ));
    }

    // Rough allocation check: width × height × 4 bytes (RGBA worst case).
    let approx_alloc = (w as u64) * (h as u64) * 4;
    if approx_alloc > MAX_ALLOC {
        return Err(PipelineError::new(
            "FS_IMAGE_DECODE",
            format!(
                "image would require ~{} MB decoded, exceeding limit of {} MB",
                approx_alloc / 1_048_576,
                MAX_ALLOC / 1_048_576,
            ),
        ));
    }

    // Step 2: full decode — safe now that dimensions are checked.
    image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| PipelineError::new("FS_IMAGE_DECODE", format!("format detection: {e}")))?
        .decode()
        .map_err(|e| PipelineError::new("FS_IMAGE_DECODE", format!("image decode: {e}")))
}
