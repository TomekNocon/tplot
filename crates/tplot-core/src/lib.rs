//! Data ingestion, layout, and rasterization for the tplot toolchain.

pub mod dataframe;
pub use dataframe::{Column, DataFrame, DataFrameError, Series};

pub mod input;

pub mod pixel_buffer;
pub use pixel_buffer::PixelBuffer;
