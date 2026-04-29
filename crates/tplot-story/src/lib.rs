//! Storytelling-with-Data treatment pass for the tplot toolchain.

pub mod focal;
pub use focal::{pick_focal, FocalChoice, FocalResult, SeriesPoint};

pub mod palette;
pub use palette::build_palette_map;
