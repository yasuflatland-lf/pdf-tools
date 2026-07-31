//! JPEG frame-header inspection, free of PDFium and of any decoding.
//!
//! The composer needs one decision before it hands a file to PDFium: can the
//! original stream be embedded as DCTDecode, or must the image be decoded and
//! re-embedded as a bitmap? Getting that wrong is not a recoverable error --
//! PDFium accepts a CMYK stream and renders it with the wrong colours -- so the
//! decision is made here, from the bytes, before PDFium ever sees them.

/// Whether a JPEG's original stream can be embedded verbatim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Passthrough {
    Eligible,
    Ineligible,
}

/// Decides whether the given bytes are a JPEG whose stream PDF can carry as-is.
///
/// Eligible: SOF0 (baseline), SOF1 (extended sequential) or SOF2 (progressive)
/// carrying one or three components -- grayscale and YCbCr, which PDF expresses
/// directly as DeviceGray and DeviceRGB.
///
/// Ineligible: four components (CMYK or YCCK), SOF3 and above (lossless,
/// differential and arithmetic-coded frames), and anything this cannot parse.
/// Every ineligible input falls back to the raster path, which handles it the
/// way it always has.
pub fn passthrough(bytes: &[u8]) -> Passthrough {
    match frame_header(bytes) {
        Some((0xC0..=0xC2, components)) => {
            if matches!(components, 1 | 3) {
                Passthrough::Eligible
            } else {
                Passthrough::Ineligible
            }
        }
        _ => Passthrough::Ineligible,
    }
}

/// Walks the marker segments and returns the frame header's marker byte and
/// component count, or None when the input is not a JPEG, ends early, or reaches
/// its scan without declaring a frame.
fn frame_header(bytes: &[u8]) -> Option<(u8, u8)> {
    if bytes.len() < 4 || bytes[0] != 0xFF || bytes[1] != 0xD8 {
        return None;
    }

    let mut index = 2;
    while index + 1 < bytes.len() {
        if bytes[index] != 0xFF {
            // Out of step with the segment structure; nothing further can be
            // trusted, and guessing is what would embed the wrong colours.
            return None;
        }

        let marker = bytes[index + 1];
        // Any number of 0xFF bytes may pad the gap before a marker.
        if marker == 0xFF {
            index += 1;
            continue;
        }
        // Standalone markers carry no length field.
        if marker == 0x01 || (0xD0..=0xD9).contains(&marker) {
            index += 2;
            continue;
        }
        // The scan begins; a frame header should already have appeared.
        if marker == 0xDA {
            return None;
        }

        let length = u16::from_be_bytes([*bytes.get(index + 2)?, *bytes.get(index + 3)?]) as usize;
        if length < 2 {
            return None;
        }
        let segment_end = index.checked_add(2)?.checked_add(length)?;
        if segment_end > bytes.len() {
            return None;
        }

        if is_frame_header(marker) {
            // Payload: precision(1) height(2) width(2) component count(1).
            return Some((marker, *bytes.get(index + 9)?));
        }

        index = segment_end;
    }

    None
}

/// True for SOF0..SOF15. Three markers share that range without being frame
/// headers: DHT (0xC4), JPG (0xC8) and DAC (0xCC).
fn is_frame_header(marker: u8) -> bool {
    (0xC0..=0xCF).contains(&marker) && marker != 0xC4 && marker != 0xC8 && marker != 0xCC
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a minimal but structurally valid JPEG carrying the given frame
    /// marker and component count. Only the marker structure matters here; no
    /// entropy-coded data follows the scan header.
    fn jpeg_with_frame(marker: u8, components: u8) -> Vec<u8> {
        let mut bytes = vec![0xFF, 0xD8];
        // APP0 with a two-byte payload, so the scan has a segment to skip.
        bytes.extend_from_slice(&[0xFF, 0xE0, 0x00, 0x04, 0x00, 0x00]);
        bytes.extend_from_slice(&[0xFF, marker]);
        bytes.extend_from_slice(&(8 + 3 * components as u16).to_be_bytes());
        bytes.push(0x08); // sample precision
        bytes.extend_from_slice(&600u16.to_be_bytes()); // height
        bytes.extend_from_slice(&800u16.to_be_bytes()); // width
        bytes.push(components);
        for component in 0..components {
            bytes.extend_from_slice(&[component + 1, 0x11, 0x00]);
        }
        bytes.extend_from_slice(&[0xFF, 0xDA, 0x00, 0x02]);
        bytes
    }

    #[test]
    fn baseline_and_progressive_frames_with_one_or_three_components_pass_through() {
        for marker in [0xC0, 0xC1, 0xC2] {
            for components in [1, 3] {
                assert_eq!(
                    passthrough(&jpeg_with_frame(marker, components)),
                    Passthrough::Eligible,
                    "SOF{:X} with {components} components should pass through",
                    marker - 0xC0
                );
            }
        }
    }

    #[test]
    fn a_four_component_frame_never_passes_through() {
        // PDFium accepts a CMYK stream and renders it with the wrong colours,
        // so this is a routing decision, not an error path.
        for marker in [0xC0, 0xC1, 0xC2] {
            assert_eq!(
                passthrough(&jpeg_with_frame(marker, 4)),
                Passthrough::Ineligible
            );
        }
    }

    #[test]
    fn frames_above_sof2_never_pass_through() {
        for marker in [0xC3, 0xC5, 0xC6, 0xC7, 0xC9, 0xCA, 0xCB, 0xCD, 0xCE, 0xCF] {
            assert_eq!(
                passthrough(&jpeg_with_frame(marker, 3)),
                Passthrough::Ineligible,
                "marker {marker:#X} should not pass through"
            );
        }
    }

    #[test]
    fn a_frame_header_is_not_confused_with_the_markers_that_share_its_range() {
        // DHT, JPG and DAC sit inside 0xC0..=0xCF but are not frame headers.
        let mut bytes = vec![0xFF, 0xD8];
        for marker in [0xC4, 0xC8, 0xCC] {
            bytes.extend_from_slice(&[0xFF, marker, 0x00, 0x04, 0x00, 0x00]);
        }
        bytes.extend_from_slice(&jpeg_with_frame(0xC0, 3)[2..]);
        assert_eq!(passthrough(&bytes), Passthrough::Eligible);
    }

    #[test]
    fn input_that_is_not_a_jpeg_never_passes_through() {
        assert_eq!(passthrough(b""), Passthrough::Ineligible);
        assert_eq!(passthrough(b"\x89PNG\r\n\x1a\n"), Passthrough::Ineligible);
        assert_eq!(passthrough(&[0xFF, 0xD8]), Passthrough::Ineligible);
    }

    #[test]
    fn a_file_truncated_before_its_frame_header_never_passes_through() {
        let full = jpeg_with_frame(0xC0, 3);
        for length in 3..full.len() - 4 {
            assert_eq!(
                passthrough(&full[..length]),
                Passthrough::Ineligible,
                "a {length}-byte prefix should not pass through"
            );
        }
    }

    #[test]
    fn a_scan_reached_before_any_frame_header_never_passes_through() {
        let bytes = vec![0xFF, 0xD8, 0xFF, 0xDA, 0x00, 0x02];
        assert_eq!(passthrough(&bytes), Passthrough::Ineligible);
    }

    #[test]
    fn the_committed_baseline_fixture_passes_through() {
        let bytes = std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sample.jpg"),
        )
        .expect("the sample.jpg fixture is committed");
        assert_eq!(passthrough(&bytes), Passthrough::Eligible);
    }
}
