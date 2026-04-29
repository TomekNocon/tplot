use crate::pipeline::detected_terminal_size;
use anyhow::{Result, anyhow};
use tplot_core::PixelBuffer;
use tplot_core::dataframe::DataFrame;
use tplot_core::layout::layout_boxplot;
use tplot_core::rasterize::rasterize_boxplot;
use tplot_protocol::{Capabilities, FocusMode, Palette, StoryConfig};
use tplot_render::render_halfblocks;
use tplot_story::{boxplot_takeaway, focal::SeriesSpread, run_boxplot_story_pass};

#[derive(Debug, Clone)]
pub struct BoxOptions {
    pub x: String,
    pub y: String,
    pub focus: Option<String>,
    pub annotate: Option<String>,
    pub neutral: bool,
    pub no_takeaway: bool,
    pub width: Option<usize>,
    pub height: usize,
    pub palette_name: String,
}

pub fn render_boxplot(df: &DataFrame, opts: &BoxOptions) -> Result<String> {
    let palette = Palette::from_name(&opts.palette_name).map_err(|e| anyhow!(e.to_string()))?;
    let (canvas_w, _) = detected_terminal_size(opts.width);
    let canvas_h = opts.height;

    // ----- layout (also computes 5-number summaries) ----------------------
    let layout = layout_boxplot(df, &opts.x, &opts.y, canvas_w, canvas_h)
        .map_err(|e| anyhow!(e.to_string()))?;

    // ----- story-pass ------------------------------------------------------
    let spreads: Vec<SeriesSpread> = layout
        .boxes
        .iter()
        .map(|b| SeriesSpread {
            key: b.label.clone(),
            iqr: b.summary.iqr(),
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
    let story = run_boxplot_story_pass(&spreads, &story_cfg, palette);

    // ----- rasterize -------------------------------------------------------
    let mut buf = PixelBuffer::new(layout.plot_box.pixel_width, layout.plot_box.pixel_height);
    rasterize_boxplot(
        &layout,
        story.focal.as_deref(),
        &story.palette_map,
        &mut buf,
    );

    // ----- render ----------------------------------------------------------
    let caps = Capabilities::from_vars(|name| std::env::var(name).ok());
    let body = render_halfblocks(&buf, caps);
    let body_lines: Vec<&str> = body.lines().collect();

    // ----- compose: y-axis labels, plot, x-axis labels, takeaway ----------
    let mut out = String::new();
    let n_rows = body_lines.len();
    for (i, line) in body_lines.iter().enumerate() {
        let y_label = if i == 0 {
            format!("{:>5.0}", layout.y_max)
        } else if n_rows > 1 && i == n_rows / 2 {
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

    // X-axis labels under each box.
    let mut x_axis = String::new();
    x_axis.push_str(&" ".repeat(layout.left_margin));
    let leading = layout
        .boxes
        .first()
        .map(|b| b.pixel_x.saturating_sub(layout.bar_cell_width / 2))
        .unwrap_or(0);
    x_axis.push_str(&" ".repeat(leading));
    for (idx, el) in layout.boxes.iter().enumerate() {
        let trimmed: String = el.label.chars().take(layout.bar_cell_width).collect();
        let label_chars = trimmed.chars().count();
        let pad_left = layout.bar_cell_width.saturating_sub(label_chars) / 2;
        let pad_right = layout.bar_cell_width.saturating_sub(label_chars + pad_left);
        x_axis.push_str(&" ".repeat(pad_left));
        x_axis.push_str(&trimmed);
        x_axis.push_str(&" ".repeat(pad_right));
        if idx + 1 < layout.boxes.len() {
            x_axis.push_str(&" ".repeat(layout.gap_cell_width));
        }
    }
    out.push_str(&x_axis);
    out.push('\n');

    // ----- takeaway --------------------------------------------------------
    if !opts.no_takeaway {
        out.push('\n');
        out.push_str(&" ".repeat(layout.left_margin + 1));
        let takeaway = if let Some(custom) = &opts.annotate {
            custom.clone()
        } else if let Some(name) = &story.focal {
            let el = layout.boxes.iter().find(|b| b.label == *name);
            if let Some(el) = el {
                boxplot_takeaway(
                    Some(name),
                    el.summary.q1,
                    el.summary.q3,
                    el.summary.min,
                    el.summary.max,
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
    fn renders_boxplot_with_widest_iqr_focal() {
        // /orders has clearly wider IQR than /users.
        let csv = "endpoint,ms\n\
            /users,48\n/users,50\n/users,51\n/users,52\n/users,53\n\
            /orders,30\n/orders,60\n/orders,100\n/orders,250\n/orders,400\n/orders,80\n";
        let df = parse_csv_str(csv).unwrap();
        let opts = BoxOptions {
            x: "endpoint".into(),
            y: "ms".into(),
            focus: None,
            annotate: None,
            neutral: false,
            no_takeaway: false,
            width: Some(80),
            height: 16,
            palette_name: "signature".into(),
        };
        let out = render_boxplot(&df, &opts).unwrap();
        // /orders should be focal → burnt orange escape.
        assert!(out.contains("\x1b[38;2;238;123;61m"), "missing focal color");
        assert!(out.contains("/orders"));
        // Takeaway about the widest spread.
        assert!(out.to_lowercase().contains("widest") || out.to_lowercase().contains("spread"));
    }
}
