use crate::pipeline::require_minimum_width;
use anyhow::{Result, anyhow};
use tplot_core::PixelBuffer;
use tplot_core::dataframe::DataFrame;
use tplot_core::layout::layout_stacked_area;
use tplot_core::rasterize::rasterize_stacked_area;
use tplot_protocol::{Capabilities, FocusMode, Palette, StoryConfig};
use tplot_render::render_halfblocks;
use tplot_story::{
    focal::SeriesTotal, run_stacked_area_story_pass_with_theme, stacked_area_takeaway,
};

#[derive(Debug, Clone)]
pub struct AreaOptions {
    pub x: String,
    pub y: String,
    pub group: String,
    pub focus: Option<String>,
    pub annotate: Option<String>,
    pub neutral: bool,
    pub no_takeaway: bool,
    pub width: Option<usize>,
    pub height: usize,
    pub palette_name: String,
}

pub fn render_stacked_area(df: &DataFrame, opts: &AreaOptions) -> Result<String> {
    let palette = Palette::from_name(&opts.palette_name).map_err(|e| anyhow!(e.to_string()))?;
    let (canvas_w, _) = require_minimum_width(opts.width)?;
    let canvas_h = opts.height;

    // ----- layout (also computes per-series totals) -----------------------
    let layout = layout_stacked_area(df, &opts.x, &opts.y, &opts.group, canvas_w, canvas_h)
        .map_err(|e| anyhow!(e.to_string()))?;

    // ----- story-pass over per-series totals ------------------------------
    let totals: Vec<SeriesTotal> = layout
        .series
        .iter()
        .map(|s| SeriesTotal {
            key: s.key.clone(),
            total: s.total,
        })
        .collect();
    let story_cfg = StoryConfig {
        enabled: !opts.neutral,
        takeaway: !opts.no_takeaway,
        focus: opts
            .focus
            .clone()
            .map(FocusMode::Series)
            .unwrap_or(FocusMode::Auto),
        annotation: opts.annotate.clone(),
    };
    let caps = Capabilities::from_vars(|name| std::env::var(name).ok());
    let story = run_stacked_area_story_pass_with_theme(&totals, &story_cfg, palette, caps.theme);

    // ----- rasterize -------------------------------------------------------
    let mut buf = PixelBuffer::new(layout.plot_box.pixel_width, layout.plot_box.pixel_height);
    rasterize_stacked_area(&layout, &story.palette_map, &mut buf);

    // ----- render ----------------------------------------------------------
    let body = render_halfblocks(&buf, caps);
    let body_lines: Vec<&str> = body.lines().collect();

    // ----- compose: y-axis labels, plot, x-axis labels, legend, takeaway --
    let mut out = String::new();
    let n_rows = body_lines.len();
    for (i, line) in body_lines.iter().enumerate() {
        let y_label = if i == 0 {
            format!("{:>5.0}", layout.y_max)
        } else if n_rows > 1 && i == n_rows / 2 {
            format!("{:>5.0}", layout.y_max / 2.0)
        } else if i + 1 == n_rows {
            format!("{:>5}", "0")
        } else {
            " ".repeat(5)
        };
        out.push_str(&y_label);
        out.push(' ');
        out.push_str(line);
        out.push('\n');
    }

    // X-axis labels: just min and max under the plot.
    let mut x_axis = String::new();
    x_axis.push_str(&" ".repeat(layout.left_margin));
    let label_l = format!("{:.0}", layout.x_min);
    let label_r = format!("{:.0}", layout.x_max);
    let plot_cells = layout.plot_box.pixel_width;
    let pad = plot_cells.saturating_sub(label_l.len() + label_r.len());
    x_axis.push_str(&label_l);
    x_axis.push_str(&" ".repeat(pad));
    x_axis.push_str(&label_r);
    out.push_str(&x_axis);
    out.push('\n');

    // Legend: list series in stack order with their focal/context indicator.
    let legend: String = layout
        .series
        .iter()
        .map(|s| {
            let focal = story.focal.as_deref() == Some(s.key.as_str());
            let marker = if focal { "\u{25A0}" } else { "\u{00B7}" };
            format!(" {marker} {}", s.key)
        })
        .collect::<Vec<_>>()
        .join("  ");
    out.push_str(&" ".repeat(layout.left_margin));
    out.push_str(&legend);
    out.push('\n');

    // ----- takeaway --------------------------------------------------------
    if !opts.no_takeaway {
        out.push('\n');
        out.push_str(&" ".repeat(layout.left_margin + 1));
        let takeaway = if let Some(custom) = &opts.annotate {
            custom.clone()
        } else if let Some(name) = &story.focal {
            let grand: f64 = layout.series.iter().map(|s| s.total).sum();
            let focal_total = layout
                .series
                .iter()
                .find(|s| s.key == *name)
                .map(|s| s.total)
                .unwrap_or(0.0);
            stacked_area_takeaway(Some(name), focal_total, grand)
        } else {
            stacked_area_takeaway(None, 0.0, 0.0)
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
    fn renders_stacked_area_with_focal_largest_total() {
        let csv = "month,rev,g\n\
            1,10,NA\n1,5,EMEA\n\
            2,20,NA\n2,5,EMEA\n\
            3,40,NA\n3,6,EMEA\n";
        let df = parse_csv_str(csv).unwrap();
        let opts = AreaOptions {
            x: "month".into(),
            y: "rev".into(),
            group: "g".into(),
            focus: None,
            annotate: None,
            neutral: false,
            no_takeaway: false,
            width: Some(80),
            height: 16,
            palette_name: "signature".into(),
        };
        let out = render_stacked_area(&df, &opts).unwrap();
        // NA total = 70, EMEA total = 16 → NA focal in burnt orange.
        assert!(out.contains("\x1b[38;2;238;123;61m"), "missing focal color");
        assert!(out.contains("NA"));
        // Takeaway phrasing.
        assert!(out.to_lowercase().contains("contributed") || out.to_lowercase().contains("most"));
    }
}
