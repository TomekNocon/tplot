//! Graphics protocol output: PixelBuffer → image → PNG → terminal escapes.

use image::{ImageBuffer, Rgba, RgbaImage};
use tplot_core::PixelBuffer;

pub mod iterm2;
pub mod kitty;
pub mod png;

/// Convert a `PixelBuffer` to an upscaled `RgbaImage`. Each sub-pixel becomes
/// an `upscale × upscale` block so the resulting PNG is large enough that
/// terminals don't render it at micro-size when shown inline.
pub fn buffer_to_image(buf: &PixelBuffer, upscale: u32) -> RgbaImage {
    let upscale = upscale.max(1);
    let pw = buf.pixel_width() as u32;
    let ph = buf.pixel_height() as u32;
    let out_w = pw * upscale;
    let out_h = ph * upscale;
    let mut img: RgbaImage = ImageBuffer::new(out_w, out_h);

    for src_y in 0..ph {
        for src_x in 0..pw {
            let pixel: Rgba<u8> = match buf.get(src_x as usize, src_y as usize) {
                Some(c) => Rgba([c.r, c.g, c.b, 255]),
                None => Rgba([0, 0, 0, 0]),
            };
            for dy in 0..upscale {
                for dx in 0..upscale {
                    img.put_pixel(src_x * upscale + dx, src_y * upscale + dy, pixel);
                }
            }
        }
    }
    img
}

#[cfg(test)]
mod tests {
    use super::*;
    use tplot_core::PixelBuffer;
    use tplot_protocol::RgbColor;

    #[test]
    fn empty_buffer_yields_fully_transparent_image() {
        let buf = PixelBuffer::new(4, 4);
        let img = buffer_to_image(&buf, 1);
        assert_eq!(img.width(), 4);
        assert_eq!(img.height(), 4);
        // Every pixel should have alpha = 0.
        for x in 0..4 {
            for y in 0..4 {
                let pixel = img.get_pixel(x, y);
                assert_eq!(pixel[3], 0, "pixel ({x},{y}) should be transparent");
            }
        }
    }

    #[test]
    fn set_pixel_yields_opaque_rgba() {
        let mut buf = PixelBuffer::new(2, 2);
        let red = RgbColor { r: 255, g: 0, b: 0 };
        buf.set(0, 0, red);
        let img = buffer_to_image(&buf, 1);
        let p = img.get_pixel(0, 0);
        assert_eq!(p[0], 255);
        assert_eq!(p[1], 0);
        assert_eq!(p[2], 0);
        assert_eq!(p[3], 255);
    }

    #[test]
    fn upscale_factor_4_quadruples_dimensions() {
        let buf = PixelBuffer::new(2, 3);
        let img = buffer_to_image(&buf, 4);
        assert_eq!(img.width(), 8);
        assert_eq!(img.height(), 12);
    }
}
