use crate::pipeline::{require_minimum_width, resolve_graphics};
use anyhow::{Result, anyhow};
use tplot_core::PixelBuffer;
use tplot_core::dataframe::DataFrame;
use tplot_core::layout::layout_ridgeline;
use tplot_core::rasterize::rasterize_ridgeline;
use tplot_protocol::{Capabilities, FocusMode, GraphicsProtocol, Palette, StoryConfig};
use tplot_render::render_halfblocks;
use tplot_story::{boxplot_takeaway, focal::SeriesSpread, run_boxplot_story_pass_with_theme};

#[derive(Debug, Clone)]
pub struct RidgeOptions {
    pub x: String,
    pub group: String,
    pub focus: Option<String>,
    pub annotate: Option<String>,
    pub neutral: bool,
    pub no_takeaway: bool,
    pub graphics: String,
    pub width: Option<usize>,
    pub height: usize,
    pub palette_name: String,
}

pub fn render_ridgeline(df: &DataFrame, opts: &RidgeOptions) -> Result<String> {
    let palette = Palette::from_name(&opts.palette_name).map_err(|e| anyhow!(e.to_string()))?;
    let (canvas_w, _) = require_minimum_width(opts.width)?;
    let canvas_h = opts.height;

    let layout = layout_ridgeline(df, &opts.x, &opts.group, canvas_w, canvas_h)
        .map_err(|e| anyhow!(e.to_string()))?;

    // Focal-by-IQR (same heuristic as box plot / violin).
    let spreads: Vec<SeriesSpread> = layout
        .ridges
        .iter()
        .map(|r| SeriesSpread {
            key: r.label.clone(),
            iqr: r.summary.iqr(),
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
    let story = run_boxplot_story_pass_with_theme(&spreads, &story_cfg, palette, caps.theme);

    // Rasterize.
    let mut buf = PixelBuffer::new(layout.plot_box.pixel_width, layout.plot_box.pixel_height);
    rasterize_ridgeline(
        &layout,
        story.focal.as_deref(),
        &story.palette_map,
        &mut buf,
    );

    // Graphics path.
    let protocol = resolve_graphics(&opts.graphics, caps);
    if protocol != GraphicsProtocol::None {
        let mut out = Vec::new();
        out.extend(tplot_render::graphics::render_graphics(&buf, protocol, 6));
        out.push(b'\n');
        return Ok(String::from_utf8_lossy(&out).to_string());
    }

    // Text path: render to half-blocks, prepend group labels per row.
    let body = render_halfblocks(&buf, caps);
    let body_lines: Vec<&str> = body.lines().collect();

    // Determine which CELL row each ridge's baseline lives on.
    let baseline_cell_rows: Vec<usize> = layout.ridges.iter().map(|r| r.baseline_y / 2).collect();

    let mut out = String::new();
    let label_w = layout.left_margin.saturating_sub(2);
    for (cy, line) in body_lines.iter().enumerate() {
        // Find a ridge whose baseline is on this cell row.
        let ridge_label = layout
            .ridges
            .iter()
            .enumerate()
            .find(|(i, _)| baseline_cell_rows[*i] == cy)
            .map(|(_, r)| r.label.as_str());
        if let Some(label) = ridge_label {
            let trimmed: String = label.chars().take(label_w).collect();
            out.push_str(&format!("{:>w$}  ", trimmed, w = label_w));
        } else {
            out.push_str(&" ".repeat(layout.left_margin));
        }
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

    // Takeaway.
    if !opts.no_takeaway {
        out.push('\n');
        out.push_str(&" ".repeat(layout.left_margin + 1));
        let takeaway = if let Some(custom) = &opts.annotate {
            custom.clone()
        } else if let Some(name) = &story.focal {
            let r = layout.ridges.iter().find(|r| r.label == *name);
            if let Some(r) = r {
                boxplot_takeaway(
                    Some(name),
                    r.summary.q1,
                    r.summary.q3,
                    r.summary.min,
                    r.summary.max,
                )
            } else {
                boxplot_takeaway(None, 0.0, 0.0, 0.0, 0.0)
            }
        } else {
            boxplot_takeaway(None, 0.0, 0.0, 0.0, 0.0)
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
    fn renders_ridgeline_with_widest_iqr_focal() {
        // jan: tight cluster ~50, feb: wider spread, mar: very wide.
        let csv = "month,ms\n\
            jan,40\njan,50\njan,55\njan,60\n\
            feb,30\nfeb,45\nfeb,80\nfeb,100\nfeb,130\nfeb,160\n\
            mar,5\nmar,40\nmar,80\nmar,150\nmar,250\nmar,300\nmar,350\n";
        let df = parse_csv_str(csv).unwrap();
        let opts = RidgeOptions {
            x: "ms".into(),
            group: "month".into(),
            focus: None,
            annotate: None,
            neutral: false,
            no_takeaway: false,
            graphics: "none".into(),
            width: Some(80),
            height: 16,
            palette_name: "signature".into(),
        };
        let out = render_ridgeline(&df, &opts).unwrap();
        // mar has the widest IQR → focal in burnt orange.
        assert!(out.contains("\x1b[38;2;238;123;61m"), "missing focal color");
        // Group labels should appear on the left.
        assert!(out.contains("jan"));
        assert!(out.contains("feb"));
        assert!(out.contains("mar"));
    }
}
