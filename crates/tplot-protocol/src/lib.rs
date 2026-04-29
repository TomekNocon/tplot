//! Shared types for the tplot toolchain.

pub mod color;
pub use color::RgbColor;

pub mod palette;
pub use palette::Palette;

pub mod capabilities;
pub use capabilities::{Capabilities, ColorDepth, GlyphSet, GraphicsProtocol};

pub mod chart;
pub mod story;
pub use chart::{Axis, BarOrientation, ChartKind, ChartSpec};
pub use story::{FocusMode, StoryConfig};

pub mod heat_ramp;
pub use heat_ramp::HeatRamp;
