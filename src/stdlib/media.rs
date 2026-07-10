//! Native media (image) builtins.
//!
//! These functions let WFL programs decode, inspect, and resize images without
//! shelling out to an external tool — the bytes come from `read binary content`
//! (or a binary HTTP body) and go back out as a [`Value::Binary`] that can be
//! written to a file or sent as a web response. This keeps image handling inside
//! the language instead of at a process boundary.

use super::helpers::{check_arg_count, expect_binary, expect_number};
use crate::interpreter::environment::Environment;
use crate::interpreter::error::RuntimeError;
use crate::interpreter::value::Value;
use image::{DynamicImage, ImageFormat, ImageReader, Limits, imageops::FilterType};
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Cursor;
use std::rc::Rc;
use std::sync::Arc;

/// Upper bound on a decoded image's width or height, in pixels. Larger inputs
/// are rejected before decoding to guard against decompression bombs.
const MAX_DIMENSION: u32 = 30_000;

/// Upper bound on total pixels (width × height) for a resize target, ~100
/// megapixels. Prevents a small request from allocating an enormous buffer.
const MAX_PIXELS: u64 = 100_000_000;

/// Upper bound on the memory the decoder may allocate while decoding (512 MiB).
const MAX_DECODE_ALLOC: u64 = 512 * 1024 * 1024;

/// Validate and extract a positive pixel dimension (width or height) argument.
///
/// WFL numbers are `f64`; a dimension must be a finite, positive whole number
/// within [`MAX_DIMENSION`]. Non-integers and out-of-range values produce a
/// clear, actionable error rather than a silent truncation.
fn expect_dimension(func_name: &str, value: &Value, label: &str) -> Result<u32, RuntimeError> {
    let n = expect_number(value)?;
    if !n.is_finite() || n.fract() != 0.0 || n < 1.0 || n > MAX_DIMENSION as f64 {
        return Err(RuntimeError::new(
            format!(
                "{func_name}: {label} must be a whole number between 1 and {MAX_DIMENSION}, got {n}"
            ),
            0,
            0,
        ));
    }
    Ok(n as u32)
}

/// Decode image bytes with resource limits applied, returning both the decoded
/// image and the format it was stored in (so it can be re-encoded to the same
/// format).
fn decode_with_limits(
    func_name: &str,
    bytes: &[u8],
) -> Result<(DynamicImage, ImageFormat), RuntimeError> {
    let reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| RuntimeError::new(format!("{func_name}: could not read image: {e}"), 0, 0))?;

    let format = reader.format().ok_or_else(|| {
        RuntimeError::new(
            format!("{func_name}: unrecognized or unsupported image format"),
            0,
            0,
        )
    })?;

    let mut limits = Limits::no_limits();
    limits.max_image_width = Some(MAX_DIMENSION);
    limits.max_image_height = Some(MAX_DIMENSION);
    limits.max_alloc = Some(MAX_DECODE_ALLOC);
    let mut reader = reader;
    reader.limits(limits);

    let image = reader.decode().map_err(|e| {
        RuntimeError::new(format!("{func_name}: could not decode image: {e}"), 0, 0)
    })?;

    Ok((image, format))
}

/// Encode a [`DynamicImage`] back to the given format's bytes.
///
/// JPEG has no alpha channel, so an RGBA image is flattened to RGB first;
/// otherwise the image is written as-is. Formats without a built-in encoder
/// produce a clear error naming the format.
fn encode_to_format(
    func_name: &str,
    image: &DynamicImage,
    format: ImageFormat,
) -> Result<Vec<u8>, RuntimeError> {
    let mut out = Vec::new();
    let result = if format == ImageFormat::Jpeg {
        // JpegEncoder rejects images with an alpha channel; drop it.
        DynamicImage::ImageRgb8(image.to_rgb8()).write_to(&mut Cursor::new(&mut out), format)
    } else {
        image.write_to(&mut Cursor::new(&mut out), format)
    };

    result.map_err(|e| {
        RuntimeError::new(
            format!(
                "{func_name}: could not encode image as {}: {e}",
                format.extensions_str().first().copied().unwrap_or("?")
            ),
            0,
            0,
        )
    })?;
    Ok(out)
}

/// `resize_image of image_data and width and height` -> resized image bytes.
///
/// Decodes `image_data` (any supported format — PNG, JPEG, GIF, BMP, WebP),
/// scales it to fit within `width` × `height` **preserving the aspect ratio**
/// (the result is never distorted and never larger than the box on either
/// axis), and re-encodes it to the original format. Returns the new image as
/// binary data.
///
/// Usage:
/// ```wfl
/// open file at "photo.jpg" for reading binary as source
/// store original as read binary from source
/// close file source
/// store thumbnail as resize_image of original and 200 and 200
/// ```
pub fn native_resize_image(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("resize_image", &args, 3)?;
    let bytes = expect_binary(&args[0])?;
    let width = expect_dimension("resize_image", &args[1], "width")?;
    let height = expect_dimension("resize_image", &args[2], "height")?;

    if width as u64 * height as u64 > MAX_PIXELS {
        return Err(RuntimeError::new(
            format!(
                "resize_image: target size {width}x{height} exceeds the {MAX_PIXELS}-pixel limit"
            ),
            0,
            0,
        ));
    }

    let (image, format) = decode_with_limits("resize_image", &bytes)?;
    // `resize` fits the image inside the box while preserving aspect ratio.
    // Lanczos3 gives high-quality downscaling, the common case for thumbnails.
    let resized = image.resize(width, height, FilterType::Lanczos3);
    let out = encode_to_format("resize_image", &resized, format)?;
    Ok(Value::Binary(Arc::from(out)))
}

/// `image_dimensions of image_data` -> object with `width` and `height` in pixels.
///
/// Reads only the image header, so it is cheap even for large images. Useful for
/// deciding whether a resize is needed or computing a target size.
///
/// Usage:
/// ```wfl
/// store size as image_dimensions of original
/// display size["width"]
/// display size["height"]
/// ```
pub fn native_image_dimensions(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("image_dimensions", &args, 1)?;
    let bytes = expect_binary(&args[0])?;

    let reader = ImageReader::new(Cursor::new(&*bytes))
        .with_guessed_format()
        .map_err(|e| {
            RuntimeError::new(format!("image_dimensions: could not read image: {e}"), 0, 0)
        })?;

    let (width, height) = reader.into_dimensions().map_err(|e| {
        RuntimeError::new(
            format!("image_dimensions: could not read image dimensions: {e}"),
            0,
            0,
        )
    })?;

    let mut map = HashMap::new();
    map.insert("width".to_string(), Value::Number(width as f64));
    map.insert("height".to_string(), Value::Number(height as f64));
    Ok(Value::Object(Rc::new(RefCell::new(map))))
}

pub fn register_media(env: &mut Environment) {
    env.define_native("resize_image", native_resize_image);
    env.define_native("image_dimensions", native_image_dimensions);
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};

    /// Build a solid-colour test image encoded in the given format.
    fn make_image(width: u32, height: u32, format: ImageFormat) -> Vec<u8> {
        let img = RgbaImage::from_pixel(width, height, Rgba([10, 120, 200, 255]));
        let mut bytes = Vec::new();
        DynamicImage::ImageRgba8(img)
            .write_to(&mut Cursor::new(&mut bytes), format)
            .expect("encode test image");
        bytes
    }

    fn binary(bytes: Vec<u8>) -> Value {
        Value::Binary(Arc::from(bytes))
    }

    #[test]
    fn resize_png_preserves_aspect_ratio_and_format() {
        // 100x50 source, fit into a 20x20 box -> width-bound to 20x10.
        let src = make_image(100, 50, ImageFormat::Png);
        let out = native_resize_image(vec![binary(src), Value::Number(20.0), Value::Number(20.0)])
            .unwrap();
        let out_bytes = match out {
            Value::Binary(b) => b,
            other => panic!("expected binary, got {other:?}"),
        };
        // Output stays PNG.
        assert_eq!(
            image::guess_format(&out_bytes).unwrap(),
            ImageFormat::Png,
            "format should be preserved"
        );
        let (w, h) = ImageReader::new(Cursor::new(&*out_bytes))
            .with_guessed_format()
            .unwrap()
            .into_dimensions()
            .unwrap();
        assert_eq!((w, h), (20, 10));
    }

    #[test]
    fn resize_jpeg_with_alpha_source_succeeds() {
        // JPEG cannot store alpha; the encoder path must flatten RGBA.
        let src = make_image(80, 40, ImageFormat::Jpeg);
        let out = native_resize_image(vec![binary(src), Value::Number(40.0), Value::Number(40.0)])
            .unwrap();
        let out_bytes = match out {
            Value::Binary(b) => b,
            other => panic!("expected binary, got {other:?}"),
        };
        assert_eq!(image::guess_format(&out_bytes).unwrap(), ImageFormat::Jpeg);
    }

    #[test]
    fn resize_never_upscales_beyond_box() {
        // Fitting a 10x10 image into 200x200 keeps it at 10x10 (resize scales
        // down to fit but the box is larger, so dimensions are unchanged).
        let src = make_image(10, 10, ImageFormat::Png);
        let out = native_resize_image(vec![
            binary(src),
            Value::Number(200.0),
            Value::Number(200.0),
        ])
        .unwrap();
        let out_bytes = match out {
            Value::Binary(b) => b,
            other => panic!("expected binary, got {other:?}"),
        };
        let (w, h) = ImageReader::new(Cursor::new(&*out_bytes))
            .with_guessed_format()
            .unwrap()
            .into_dimensions()
            .unwrap();
        assert_eq!((w, h), (200, 200));
    }

    #[test]
    fn image_dimensions_reports_source_size() {
        let src = make_image(123, 45, ImageFormat::Png);
        let out = native_image_dimensions(vec![binary(src)]).unwrap();
        let obj = match out {
            Value::Object(o) => o,
            other => panic!("expected object, got {other:?}"),
        };
        let obj = obj.borrow();
        assert!(matches!(obj.get("width"), Some(Value::Number(n)) if *n == 123.0));
        assert!(matches!(obj.get("height"), Some(Value::Number(n)) if *n == 45.0));
    }

    #[test]
    fn resize_rejects_non_integer_dimension() {
        let src = make_image(10, 10, ImageFormat::Png);
        let err = native_resize_image(vec![binary(src), Value::Number(20.5), Value::Number(20.0)])
            .unwrap_err();
        assert!(err.message.contains("whole number"), "got: {}", err.message);
    }

    #[test]
    fn resize_rejects_zero_dimension() {
        let src = make_image(10, 10, ImageFormat::Png);
        let err = native_resize_image(vec![binary(src), Value::Number(0.0), Value::Number(20.0)])
            .unwrap_err();
        assert!(err.message.contains("between 1"), "got: {}", err.message);
    }

    #[test]
    fn resize_rejects_non_image_bytes() {
        let err = native_resize_image(vec![
            binary(b"not an image at all".to_vec()),
            Value::Number(20.0),
            Value::Number(20.0),
        ])
        .unwrap_err();
        assert!(err.message.contains("resize_image"));
    }

    #[test]
    fn resize_rejects_non_binary_first_arg() {
        let err = native_resize_image(vec![
            Value::Text(Arc::from("hello")),
            Value::Number(20.0),
            Value::Number(20.0),
        ])
        .unwrap_err();
        assert!(err.message.contains("binary data"), "got: {}", err.message);
    }
}
