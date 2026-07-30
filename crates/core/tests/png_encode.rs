use image::ImageReader;
use pdf_tools_core::application::errors::ImageError;
use pdf_tools_core::domain::geometry::RasterImage;
use pdf_tools_core::infrastructure::png::encode_png;
use std::io::Cursor;

const PNG_MAGIC: &[u8; 8] = b"\x89PNG\r\n\x1a\n";

#[test]
fn encodes_a_raster_image_as_png() {
    let image = RasterImage {
        width: 2,
        height: 2,
        rgba: vec![255; 16],
    };

    let bytes = encode_png(&image).unwrap();

    assert_eq!(&bytes[..PNG_MAGIC.len()], PNG_MAGIC);
}

#[test]
fn encoded_png_round_trips_dimensions_and_pixels() {
    let image = RasterImage {
        width: 2,
        height: 2,
        rgba: vec![
            255, 0, 0, 255, 0, 255, 0, 128, 0, 0, 255, 64, 255, 255, 255, 0,
        ],
    };

    let bytes = encode_png(&image).unwrap();
    let decoded = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .unwrap()
        .decode()
        .unwrap()
        .to_rgba8();

    assert_eq!(decoded.width(), image.width);
    assert_eq!(decoded.height(), image.height);
    assert_eq!(decoded.as_raw(), &image.rgba);
}

#[test]
fn rejects_a_raster_with_an_invalid_buffer_length() {
    let image = RasterImage {
        width: 2,
        height: 2,
        rgba: vec![255; 15],
    };

    assert!(matches!(
        encode_png(&image),
        Err(ImageError::EncodeFailed { .. })
    ));
}

#[test]
fn rejects_zero_dimensions() {
    let image = RasterImage {
        width: 0,
        height: 2,
        rgba: Vec::new(),
    };

    assert!(matches!(
        encode_png(&image),
        Err(ImageError::EncodeFailed { .. })
    ));
}
