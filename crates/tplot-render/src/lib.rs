//! Pixel buffer to ANSI string rendering for the tplot toolchain.

pub mod ansi;
pub use ansi::{bg, fg, reset};

pub mod halfblocks;
pub use halfblocks::render_halfblocks;

pub mod vertical_blocks;
pub use vertical_blocks::render_vertical_blocks;

pub mod braille;
pub use braille::render_braille;
