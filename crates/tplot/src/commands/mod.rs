pub mod bar;
pub use bar::{RenderOptions, render_bar};

pub mod histogram;
pub use histogram::{HistogramOptions, render_histogram};

pub mod json;
pub use json::render_from_json;
