//! PNG encoding helper.

use image::RgbaImage;
use std::io::Cursor;

/// Encode an `RgbaImage` to PNG bytes.
pub fn encode_png(img: &RgbaImage) -> Result<Vec<u8>, image::ImageError> {
    let mut cursor = Cursor::new(Vec::with_capacity(4096));
    img.write_to(&mut cursor, image::ImageFormat::Png)?;
    Ok(cursor.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgba};

    #[test]
    fn png_starts_with_png_magic_bytes() {
        let img: image::RgbaImage = ImageBuffer::from_pixel(2, 2, Rgba([0xee, 0x7b, 0x3d, 255]));
        let bytes = encode_png(&img).unwrap();
        // PNG magic: \x89 P N G \r \n \x1a \n
        assert_eq!(
            &bytes[..8],
            &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]
        );
    }
}
