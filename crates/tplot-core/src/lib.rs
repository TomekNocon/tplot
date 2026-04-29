//! Data ingestion, layout, and rasterization for the tplot toolchain.

pub mod dataframe;
pub use dataframe::{Column, DataFrame, DataFrameError, Series};
