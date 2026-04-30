//! Graphics protocol output: PixelBuffer → image → PNG → terminal escapes.

use image::{ImageBuffer, Rgba, RgbaImage};
use tplot_core::PixelBuffer;
use tplot_protocol::GraphicsProtocol;

pub mod iterm2;
pub mod kitty;
pub mod png;

/// Top-level graphics rendering. Returns an empty Vec if the protocol is
/// `None` or unsupported (Sixel, until Plan 7c) — callers should treat that
/// as "fall back to text rendering".
pub fn render_graphics(buf: &PixelBuffer, protocol: GraphicsProtocol, upscale: u32) -> Vec<u8> {
    match protocol {
        GraphicsProtocol::None | GraphicsProtocol::Sixel => Vec::new(),
        GraphicsProtocol::ITerm2 => {
            let img = buffer_to_image(buf, upscale);
            let png = match png::encode_png(&img) {
                Ok(p) => p,
                Err(_) => return Vec::new(),
            };
            iterm2::encode_iterm2(&png)
        }
        GraphicsProtocol::Kitty => {
            let img = buffer_to_image(buf, upscale);
            let png = match png::encode_png(&img) {
                Ok(p) => p,
                Err(_) => return Vec::new(),
            };
            kitty::encode_kitty(&png)
        }
    }
}

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
    use tplot_protocol::{GraphicsProtocol, RgbColor};

    #[test]
    fn render_graphics_produces_iterm2_when_protocol_is_iterm2() {
        let buf = PixelBuffer::new(4, 4);
        let bytes = render_graphics(&buf, GraphicsProtocol::ITerm2, 4);
        assert!(bytes.starts_with(b"\x1b]1337;File="));
    }

    #[test]
    fn render_graphics_produces_kitty_when_protocol_is_kitty() {
        let buf = PixelBuffer::new(4, 4);
        let bytes = render_graphics(&buf, GraphicsProtocol::Kitty, 4);
        assert!(bytes.starts_with(b"\x1b_Ga=T"));
    }

    #[test]
    fn render_graphics_returns_empty_for_none_or_sixel() {
        // None → empty (caller should fall back to text rendering).
        let buf = PixelBuffer::new(4, 4);
        assert!(render_graphics(&buf, GraphicsProtocol::None, 4).is_empty());
        // Sixel — not implemented in 7b, returns empty (caller falls back).
        assert!(render_graphics(&buf, GraphicsProtocol::Sixel, 4).is_empty());
    }

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
