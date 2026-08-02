use image::codecs::jpeg::JpegEncoder;
use image::{ExtendedColorType, ImageEncoder, Rgb, RgbImage};
use std::fs;
use std::path::Path;

const WIDTH: u32 = 48;
const HEIGHT: u32 = 32;

pub fn ensure_generated(directory: &Path) {
    fs::create_dir_all(directory).expect("fixture directory should be created");

    let pixels = asymmetric_pixels();
    write_jpeg_if_absent(&directory.join("exif-absent.jpg"), &pixels, None);
    write_jpeg_if_absent(
        &directory.join("exif-corrupt.jpg"),
        &pixels,
        Some(vec![0xff; 12]),
    );

    for orientation in 1..=8 {
        write_jpeg_if_absent(
            &directory.join(format!("exif-orientation-{orientation}.jpg")),
            &pixels,
            Some(exif_orientation(orientation)),
        );
    }
}

fn asymmetric_pixels() -> RgbImage {
    RgbImage::from_fn(WIDTH, HEIGHT, |x, y| {
        let third = WIDTH / 3;
        let half = HEIGHT / 2;
        match (x / third, y / half) {
            (0, 0) => Rgb([230, 30, 30]),
            (1, 0) => Rgb([30, 210, 40]),
            (2, 0) => Rgb([30, 60, 230]),
            (0, _) => Rgb([240, 210, 20]),
            (1, _) => Rgb([210, 30, 200]),
            (2, _) => Rgb([20, 210, 220]),
            _ => unreachable!("the image has exactly three columns"),
        }
    })
}

fn exif_orientation(orientation: u8) -> Vec<u8> {
    vec![
        b'I',
        b'I',
        42,
        0,
        8,
        0,
        0,
        0,
        1,
        0,
        0x12,
        0x01,
        3,
        0,
        1,
        0,
        0,
        0,
        orientation,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
    ]
}

fn write_jpeg_if_absent(path: &Path, pixels: &RgbImage, exif: Option<Vec<u8>>) {
    if path.exists() {
        return;
    }

    let mut bytes = Vec::new();
    let mut encoder = JpegEncoder::new_with_quality(&mut bytes, 95);
    if let Some(exif) = exif {
        encoder
            .set_exif_metadata(exif)
            .expect("JPEG should support EXIF metadata");
    }
    encoder
        .encode(pixels.as_raw(), WIDTH, HEIGHT, ExtendedColorType::Rgb8)
        .expect("EXIF JPEG fixture should encode");

    let file_name = path
        .file_name()
        .expect("fixture path should have a file name")
        .to_string_lossy();
    let temporary_path = path.with_file_name(format!("{file_name}.tmp.{}", std::process::id()));
    fs::write(&temporary_path, bytes).expect("temporary EXIF fixture should be written");
    if let Err(error) = fs::rename(&temporary_path, path) {
        if path.exists() {
            fs::remove_file(&temporary_path)
                .expect("redundant temporary EXIF fixture should be removed");
        } else {
            panic!("EXIF fixture should be renamed atomically: {error}");
        }
    }
}
