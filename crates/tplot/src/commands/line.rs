use crate::pipeline::{require_minimum_width, resolve_graphics};
use anyhow::{Result, anyhow};
use tplot_core::PixelBuffer;
use tplot_core::dataframe::{DataFrame, Series};
use tplot_core::layout::layout_line;
use tplot_core::rasterize::rasterize_line;
use tplot_protocol::{Capabilities, FocusMode, GraphicsProtocol, Palette, StoryConfig};
use tplot_render::render_braille;
use tplot_story::{SeriesTrend, run_line_story_pass_with_theme};

#[derive(Debug, Clone)]
pub struct LineOptions {
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
    pub graphics: String,
}

pub fn render_line(df: &DataFrame, opts: &LineOptions) -> Result<String> {
    let palette = Palette::from_name(&opts.palette_name).map_err(|e| anyhow!(e.to_string()))?;
    let (canvas_w, _) = require_minimum_width(opts.width)?;
    let canvas_h = opts.height;

    // ----- layout (same data, computes pixel positions for each series) ----
    let layout = layout_line(
        df,
        &opts.x,
        &opts.y,
        opts.group.as_deref(),
        canvas_w,
        canvas_h,
    )
    .map_err(|e| anyhow!(e.to_string()))?;

    // ----- compute trends per series for the story-pass ---------------------
    let trends: Vec<SeriesTrend> = layout
        .series
        .iter()
        .map(|s| {
            // First/last DATA values (not pixel coords) for the takeaway.
            let (first, last) =
                first_last_for_series(df, &opts.x, &opts.y, opts.group.as_deref(), &s.key);
            SeriesTrend {
                key: s.key.clone(),
                first,
                last,
            }
        })
        .collect();

    // ----- story-pass ------------------------------------------------------
    let story_cfg = StoryConfig {
        enabled: !opts.neutral,
        takeaway: !opts.no_takeaway,
        focus: match &opts.focus {
            Some(name) => FocusMode::Series(name.clone()),
            None => FocusMode::Auto,
        },
        annotation: opts.annotate.clone(),
    };
    let caps = Capabilities::from_vars(|name| std::env::var(name).ok());
    let story = run_line_story_pass_with_theme(&trends, &story_cfg, palette, caps.theme);

    // ----- rasterize -------------------------------------------------------
    let mut buf = PixelBuffer::new(layout.plot_box.pixel_width, layout.plot_box.pixel_height);
    rasterize_line(
        &layout,
        story.focal.as_deref(),
        &story.palette_map,
        &mut buf,
    );

    // ----- graphics path ---------------------------------------------------
    let protocol = resolve_graphics(&opts.graphics, caps);
    if protocol != GraphicsProtocol::None {
        let bytes = tplot_render::graphics::render_graphics(&buf, protocol, 6);
        let mut out = String::from_utf8_lossy(&bytes).into_owned();
        out.push('\n');
        if let Some(t) = &story.takeaway {
            out.push_str(t);
            out.push('\n');
        }
        return Ok(out);
    }

    // ----- render with Braille ---------------------------------------------
    let body = render_braille(&buf, caps);
    let body_lines: Vec<&str> = body.lines().collect();

    // ----- compose: y-axis labels, plot, x-axis labels, takeaway ----------
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

    // X-axis labels: just min and max under the plot.
    let mut x_axis = String::new();
    x_axis.push_str(&" ".repeat(layout.left_margin));
    let label_l = format!("{:.0}", layout.x_min);
    let label_r = format!("{:.0}", layout.x_max);
    let plot_cells = layout.plot_box.pixel_width / 2; // Braille: 2 sub-pixel cols per cell
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

fn first_last_for_series(
    df: &DataFrame,
    x_col: &str,
    y_col: &str,
    group_col: Option<&str>,
    key: &str,
) -> (f64, f64) {
    let xs: Vec<f64> = match df.column(x_col).map(|c| c.series()) {
        Ok(Series::Numbers(v)) => v.clone(),
        _ => return (0.0, 0.0),
    };
    let ys: Vec<f64> = match df.column(y_col).map(|c| c.series()) {
        Ok(Series::Numbers(v)) => v.clone(),
        _ => return (0.0, 0.0),
    };
    let groups: Vec<String> = if let Some(g) = group_col {
        match df.column(g).map(|c| c.series()) {
            Ok(Series::Strings(v)) => v.clone(),
            Ok(Series::Numbers(v)) => v.iter().map(|n| format!("{n}")).collect(),
            _ => return (0.0, 0.0),
        }
    } else {
        vec!["__main__".to_string(); xs.len()]
    };

    let lookup = if group_col.is_none() { "__main__" } else { key };
    let mut paired: Vec<(f64, f64)> = xs
        .iter()
        .zip(ys.iter())
        .zip(groups.iter())
        .filter(|((_, _), g)| g.as_str() == lookup)
        .map(|((x, y), _)| (*x, *y))
        .collect();
    paired.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    let first = paired.first().map(|p| p.1).unwrap_or(0.0);
    let last = paired.last().map(|p| p.1).unwrap_or(0.0);
    (first, last)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tplot_core::input::parse_csv_str;

    #[test]
    fn renders_line_with_focal_high_trend() {
        // Two series — A flat, B clearly growing. B should be focal.
        let csv = "t,v,g\n1,10,A\n2,11,A\n3,9,A\n4,10,A\n1,5,B\n2,30,B\n3,80,B\n4,200,B\n";
        let df = parse_csv_str(csv).unwrap();
        let opts = LineOptions {
            x: "t".into(),
            y: "v".into(),
            group: Some("g".into()),
            focus: None,
            annotate: None,
            neutral: false,
            no_takeaway: false,
            width: Some(80),
            height: 16,
            palette_name: "signature".into(),
            graphics: "none".into(),
        };
        let out = render_line(&df, &opts).unwrap();
        // Should highlight B in burnt orange.
        assert!(out.contains("\x1b[38;2;238;123;61m"), "missing focal color");
        // Some Braille glyph should appear.
        let has_braille = out.chars().any(|c| {
            let v = c as u32;
            (0x2801..=0x28FF).contains(&v)
        });
        assert!(has_braille, "no braille glyphs in output");
        // Takeaway names B.
        assert!(out.to_uppercase().contains('B') || out.contains("rose") || out.contains("grew"));
    }
}
