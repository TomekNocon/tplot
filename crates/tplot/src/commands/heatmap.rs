use crate::pipeline::require_minimum_width;
use anyhow::{Result, anyhow};
use tplot_core::PixelBuffer;
use tplot_core::dataframe::DataFrame;
use tplot_core::layout::layout_heatmap;
use tplot_core::rasterize::rasterize_heatmap;
use tplot_protocol::{Capabilities, HeatRamp};
use tplot_render::render_halfblocks;
use tplot_story::heatmap_takeaway;

#[derive(Debug, Clone)]
pub struct HeatmapOptions {
    pub x: String,
    pub y: String,
    pub value: String,
    pub ramp_name: String,
    pub annotate: Option<String>,
    pub no_takeaway: bool,
    pub width: Option<usize>,
    pub height: usize,
}

pub fn render_heatmap(df: &DataFrame, opts: &HeatmapOptions) -> Result<String> {
    let ramp = HeatRamp::from_name(&opts.ramp_name).map_err(|e| anyhow!(e.to_string()))?;
    let (canvas_w, _) = require_minimum_width(opts.width)?;
    let canvas_h = opts.height;

    let layout = layout_heatmap(df, &opts.x, &opts.y, &opts.value, ramp, canvas_w, canvas_h)
        .map_err(|e| anyhow!(e.to_string()))?;

    let mut buf = PixelBuffer::new(layout.plot_box.pixel_width, layout.plot_box.pixel_height);
    rasterize_heatmap(&layout, &mut buf);

    let caps = Capabilities::from_vars(|name| std::env::var(name).ok());
    let body = render_halfblocks(&buf, caps);
    let body_lines: Vec<&str> = body.lines().collect();

    let mut out = String::new();
    let label_margin = layout.left_margin;

    // Each terminal row in `body_lines` corresponds to ONE data row (since each
    // data row is 2 sub-pixels and half-blocks pack 2 sub-pixels per cell).
    for (yi, line) in body_lines.iter().enumerate() {
        if yi < layout.y_labels.len() {
            out.push_str(&format!(
                "{:>w$}  ",
                layout.y_labels[yi],
                w = label_margin.saturating_sub(2)
            ));
        } else {
            out.push_str(&" ".repeat(label_margin));
        }
        out.push_str(line);
        out.push('\n');
    }

    // X-axis labels under each column (truncate to cell width).
    let cw = layout.cell_term_width;
    out.push_str(&" ".repeat(label_margin));
    for label in layout.x_labels.iter() {
        let trimmed: String = label.chars().take(cw).collect();
        let pad_left = cw.saturating_sub(trimmed.chars().count()) / 2;
        let pad_right = cw.saturating_sub(trimmed.chars().count() + pad_left);
        out.push_str(&" ".repeat(pad_left));
        out.push_str(&trimmed);
        out.push_str(&" ".repeat(pad_right));
    }
    out.push('\n');

    // Takeaway.
    if !opts.no_takeaway {
        out.push('\n');
        out.push_str(&" ".repeat(label_margin + 1));
        let takeaway = if let Some(custom) = &opts.annotate {
            custom.clone()
        } else if let Some((xi, yi)) = layout.max_cell {
            let total: f64 = layout.cell_values.iter().flatten().filter_map(|c| *c).sum();
            heatmap_takeaway(
                Some((layout.y_labels[yi].as_str(), layout.x_labels[xi].as_str())),
                layout.max_value,
                total,
            )
        } else {
            heatmap_takeaway(None, 0.0, 0.0)
        };
        out.push_str(&takeaway);
        out.push('\n');
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tplot_core::input::parse_csv_str;

    #[test]
    fn renders_heatmap_with_takeaway() {
        let csv = "h,d,c\n9,Mon,5\n10,Mon,12\n11,Mon,8\n9,Tue,3\n10,Tue,25\n11,Tue,7\n";
        let df = parse_csv_str(csv).unwrap();
        let opts = HeatmapOptions {
            x: "h".into(),
            y: "d".into(),
            value: "c".into(),
            ramp_name: "inferno".into(),
            annotate: None,
            no_takeaway: false,
            width: Some(80),
            height: 12,
        };
        let out = render_heatmap(&df, &opts).unwrap();
        // Should include the hottest cell name.
        assert!(out.contains("10") && out.contains("Tue"));
        // Should include takeaway prefix.
        assert!(out.contains("Hottest cell"));
    }
}
