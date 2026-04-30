//! Data ingestion, layout, and rasterization for the tplot toolchain.

pub mod dataframe;
pub use dataframe::{Column, DataFrame, DataFrameError, Series};

pub mod input;

pub mod pixel_buffer;
pub use pixel_buffer::PixelBuffer;

pub mod layout;

pub mod rasterize;

pub mod stats;
pub use stats::{FiveNumberSummary, five_number_summary, quantile};

pub mod squarify;
pub use squarify::{Rect, squarify};

pub mod kde;
pub use kde::{kde_at, kde_evaluate, silverman_bandwidth};

pub mod sankey_graph;
pub use sankey_graph::{
    RawEdge, SankeyEdge, SankeyGraph, SankeyGraphError, SankeyNode, build_graph,
};

pub mod table;
pub use table::{
    ColumnType, TableColumn, TableLayout, TableLayoutConfig, TableLayoutError, layout_table,
};
