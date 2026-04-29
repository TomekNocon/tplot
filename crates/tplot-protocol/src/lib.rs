//! Shared types for the tplot toolchain.

pub mod color;
pub use color::RgbColor;

pub mod palette;
pub use palette::Palette;

pub mod capabilities;
pub use capabilities::{Capabilities, ColorDepth, GlyphSet, GraphicsProtocol};
