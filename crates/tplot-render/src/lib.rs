//! Pixel buffer to ANSI string rendering for the tplot toolchain.

pub mod ansi;
pub use ansi::{bg, fg, reset};

pub mod halfblocks;
pub use halfblocks::render_halfblocks;
