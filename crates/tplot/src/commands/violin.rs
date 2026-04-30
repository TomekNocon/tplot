use crate::pipeline::{require_minimum_width, resolve_graphics};
use anyhow::{Result, anyhow};
use tplot_core::PixelBuffer;
use tplot_core::dataframe::DataFrame;
use tplot_core::layout::layout_violin;
use tplot_core::rasterize::rasterize_violin;
use tplot_protocol::{Capabilities, FocusMode, GraphicsProtocol, Palette, StoryConfig};
use tplot_render::render_halfblocks;
use tplot_story::{boxplot_takeaway, focal::SeriesSpread, run_boxplot_story_pass_with_theme};

#[derive(Debug, Clone)]
pub struct ViolinOptions {
    pub x: String,
    pub y: String,
    pub focus: Option<String>,
    pub annotate: Option<String>,
    pub neutral: bool,
    pub no_takeaway: bool,
    pub graphics: String,
    pub width: Option<usize>,
    pub height: usize,
    pub palette_name: String,
}

pub fn render_violin(df: &DataFrame, opts: &ViolinOptions) -> Result<String> {
    let palette = Palette::from_name(&opts.palette_name).map_err(|e| anyhow!(e.to_string()))?;
    let (canvas_w, _) = require_minimum_width(opts.width)?;
    let canvas_h = opts.height;

    let layout = layout_violin(df, &opts.x, &opts.y, canvas_w, canvas_h)
        .map_err(|e| anyhow!(e.to_string()))?;

    // Story-pass: pick widest-IQR violin as focal (same as box plot).
    let spreads: Vec<SeriesSpread> = layout
        .violins
        .iter()
        .map(|v| SeriesSpread {
            key: v.label.clone(),
            iqr: v.summary.iqr(),
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
    rasterize_violin(
        &layout,
        story.focal.as_deref(),
        &story.palette_map,
        &mut buf,
    );

    // Graphics path: emit PNG, skip text composition.
    let protocol = resolve_graphics(&opts.graphics, caps);
    if protocol != GraphicsProtocol::None {
        let mut out = Vec::new();
        out.extend(tplot_render::graphics::render_graphics(&buf, protocol, 6));
        out.push(b'\n');
        return Ok(String::from_utf8_lossy(&out).to_string());
    }

    // Text path.
    let body = render_halfblocks(&buf, caps);
    let body_lines: Vec<&str> = body.lines().collect();
    let n_rows = body_lines.len();

    let mut out = String::new();
    for (i, line) in body_lines.iter().enumerate() {
        let y_label = if i == 0 {
            format!("{:>5.0}", layout.y_max)
        } else if i == n_rows / 2 {
            format!("{:>5.0}", (layout.y_min + layout.y_max) / 2.0)
        } else if i + 1 == n_rows {
            format!("{:>5.0}", layout.y_min)
        } else {
            " ".repeat(5)
        };
        out.push_str(&y_label);
        out.push(' ');
        out.push_str(line);
        out.push('\n');
    }

    // X-axis labels under each violin.
    let group_w = layout.bar_cell_width + layout.gap_cell_width;
    let mut x_axis = String::new();
    x_axis.push_str(&" ".repeat(layout.left_margin));
    let leading = layout
        .violins
        .first()
        .map(|v| v.pixel_x.saturating_sub(layout.bar_cell_width / 2))
        .unwrap_or(0);
    x_axis.push_str(&" ".repeat(leading));
    for (idx, v) in layout.violins.iter().enumerate() {
        let trimmed: String = v.label.chars().take(group_w).collect();
        let pad_left = layout
            .bar_cell_width
            .saturating_sub(trimmed.chars().count())
            / 2;
        let pad_right = layout
            .bar_cell_width
            .saturating_sub(trimmed.chars().count() + pad_left);
        x_axis.push_str(&" ".repeat(pad_left));
        x_axis.push_str(&trimmed);
        x_axis.push_str(&" ".repeat(pad_right));
        if idx + 1 < layout.violins.len() {
            x_axis.push_str(&" ".repeat(layout.gap_cell_width));
        }
    }
    out.push_str(&x_axis);
    out.push('\n');

    // Takeaway.
    if !opts.no_takeaway {
        out.push('\n');
        out.push_str(&" ".repeat(layout.left_margin + 1));
        let takeaway = if let Some(custom) = &opts.annotate {
            custom.clone()
        } else if let Some(name) = &story.focal {
            let v = layout.violins.iter().find(|x| x.label == *name);
            if let Some(v) = v {
                boxplot_takeaway(
                    Some(name),
                    v.summary.q1,
                    v.summary.q3,
                    v.summary.min,
                    v.summary.max,
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
    fn renders_violin_with_widest_iqr_focal() {
        let csv = "endpoint,ms\n\
            /short,48\n/short,50\n/short,51\n/short,52\n/short,53\n\
            /long,10\n/long,30\n/long,80\n/long,150\n/long,300\n/long,250\n/long,60\n/long,90\n";
        let df = parse_csv_str(csv).unwrap();
        let opts = ViolinOptions {
            x: "endpoint".into(),
            y: "ms".into(),
            focus: None,
            annotate: None,
            neutral: false,
            no_takeaway: false,
            graphics: "none".into(),
            width: Some(80),
            height: 16,
            palette_name: "signature".into(),
        };
        let out = render_violin(&df, &opts).unwrap();
        // /long has IQR ~190, /short has IQR ~3 → /long is focal, burnt orange.
        assert!(out.contains("\x1b[38;2;238;123;61m"), "missing focal color");
        assert!(out.contains("/long"));
        assert!(out.contains("/short"));
    }
}
