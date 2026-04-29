use crate::pipeline::detected_terminal_size;
use anyhow::{Result, anyhow};
use tplot_core::PixelBuffer;
use tplot_core::dataframe::DataFrame;
use tplot_core::layout::layout_scatter;
use tplot_core::rasterize::rasterize_scatter;
use tplot_protocol::{Capabilities, FocusMode, Palette, StoryConfig};
use tplot_render::render_braille;
use tplot_story::{SeriesPoint, run_bar_story_pass};

#[derive(Debug, Clone)]
pub struct ScatterOptions {
    pub x: String,
    pub y: String,
    pub group: Option<String>,
    pub focus: Option<String>,
    pub annotate: Option<String>,
    pub neutral: bool,
    pub no_takeaway: bool,
    pub width: Option<usize>,
    pub height: usize,
    pub palette_name: String,
}

pub fn render_scatter(df: &DataFrame, opts: &ScatterOptions) -> Result<String> {
    let palette = Palette::from_name(&opts.palette_name).map_err(|e| anyhow!(e.to_string()))?;
    let (canvas_w, _) = detected_terminal_size(opts.width);
    let canvas_h = opts.height;

    let layout = layout_scatter(
        df,
        &opts.x,
        &opts.y,
        opts.group.as_deref(),
        canvas_w,
        canvas_h,
    )
    .map_err(|e| anyhow!(e.to_string()))?;

    // Story-pass: pick focal series by point count (reuse run_bar_story_pass).
    let counts: Vec<SeriesPoint> = layout
        .series
        .iter()
        .map(|s| SeriesPoint {
            key: s.key.clone(),
            value: s.points.len() as f64,
        })
        .collect();
    let story_cfg = StoryConfig {
        enabled: !opts.neutral,
        takeaway: !opts.no_takeaway,
        focus: match &opts.focus {
            Some(name) => FocusMode::Series(name.clone()),
            None => FocusMode::Auto,
        },
        annotation: opts.annotate.clone(),
    };
    let story = run_bar_story_pass(&counts, &story_cfg, palette);

    let mut buf = PixelBuffer::new(layout.plot_box.pixel_width, layout.plot_box.pixel_height);
    rasterize_scatter(&layout, story.focal.as_deref(), &story.palette_map, &mut buf);

    let caps = Capabilities::from_vars(|name| std::env::var(name).ok());
    let body = render_braille(&buf, caps);
    let body_lines: Vec<&str> = body.lines().collect();

    let mut out = String::new();
    let n_rows = body_lines.len();
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

    let mut x_axis = String::new();
    x_axis.push_str(&" ".repeat(layout.left_margin));
    let label_l = format!("{:.0}", layout.x_min);
    let label_r = format!("{:.0}", layout.x_max);
    let plot_cells = layout.plot_box.pixel_width / 2;
    let pad = plot_cells.saturating_sub(label_l.len() + label_r.len());
    x_axis.push_str(&label_l);
    x_axis.push_str(&" ".repeat(pad));
    x_axis.push_str(&label_r);
    out.push_str(&x_axis);
    out.push('\n');

    if let Some(t) = story.takeaway {
        out.push('\n');
        out.push_str(&" ".repeat(layout.left_margin + 1));
        out.push_str(&t);
        out.push('\n');
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tplot_core::input::parse_csv_str;

    #[test]
    fn renders_grouped_scatter_with_focal() {
        let csv = "x,y,g\n1,10,A\n2,12,A\n3,9,A\n4,11,A\n5,13,A\n6,8,A\n1,50,B\n2,55,B\n";
        let df = parse_csv_str(csv).unwrap();
        let opts = ScatterOptions {
            x: "x".into(),
            y: "y".into(),
            group: Some("g".into()),
            focus: None,
            annotate: None,
            neutral: false,
            no_takeaway: false,
            width: Some(80),
            height: 16,
            palette_name: "signature".into(),
        };
        let out = render_scatter(&df, &opts).unwrap();
        // Should contain at least one Braille glyph.
        assert!(out.chars().any(|c| (0x2801..=0x28FF).contains(&(c as u32))));
        // A has 6 points, B has 2 → A should be focal (more dense).
        assert!(out.contains("\x1b[38;2;238;123;61m"));
    }
}
