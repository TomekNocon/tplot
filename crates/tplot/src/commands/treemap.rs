use crate::pipeline::{require_minimum_width, resolve_graphics};
use anyhow::{Result, anyhow};
use tplot_core::PixelBuffer;
use tplot_core::dataframe::DataFrame;
use tplot_core::layout::layout_treemap;
use tplot_core::rasterize::rasterize_treemap;
use tplot_protocol::{Capabilities, FocusMode, GraphicsProtocol, Palette, StoryConfig};
use tplot_render::render_halfblocks;
use tplot_story::{SeriesPoint, run_bar_story_pass_with_theme};

#[derive(Debug, Clone)]
pub struct TreeOptions {
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

const LABEL_MIN_W: usize = 6;
const LABEL_MIN_H: usize = 2;

pub fn render_treemap(df: &DataFrame, opts: &TreeOptions) -> Result<String> {
    let palette = Palette::from_name(&opts.palette_name).map_err(|e| anyhow!(e.to_string()))?;
    let (canvas_w, _) = require_minimum_width(opts.width)?;
    let canvas_h = opts.height;

    let layout = layout_treemap(df, &opts.x, &opts.y, canvas_w, canvas_h)
        .map_err(|e| anyhow!(e.to_string()))?;

    // Story-pass over per-leaf values (reuse the bar focal rules — focal = max).
    let series: Vec<SeriesPoint> = layout
        .leaves
        .iter()
        .map(|l| SeriesPoint {
            key: l.label.clone(),
            value: l.value,
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
    let story = run_bar_story_pass_with_theme(&series, &story_cfg, palette, caps.theme);

    // Rasterize.
    let mut buf = PixelBuffer::new(layout.plot_box.pixel_width, layout.plot_box.pixel_height);
    rasterize_treemap(
        &layout,
        story.focal.as_deref(),
        &story.palette_map,
        &mut buf,
    );

    // Graphics path: emit PNG and skip text composition.
    let protocol = resolve_graphics(&opts.graphics, caps);
    if protocol != GraphicsProtocol::None {
        let mut out = Vec::new();
        out.extend(tplot_render::graphics::render_graphics(&buf, protocol, 6));
        out.push(b'\n');
        if let Some(t) = story.takeaway {
            out.extend(t.as_bytes());
            out.push(b'\n');
        }
        return Ok(String::from_utf8_lossy(&out).to_string());
    }

    // Text path: render to half-blocks, overlay labels for big leaves, append legend.
    let body = render_halfblocks(&buf, caps);
    let body_lines: Vec<&str> = body.lines().collect();

    let mut out = String::new();
    for (cy, line) in body_lines.iter().enumerate() {
        // Find any leaf whose center row is this row AND that's big enough.
        let label_overlay = layout
            .leaves
            .iter()
            .filter(|l| l.cell_w >= LABEL_MIN_W && l.cell_h >= LABEL_MIN_H)
            .find(|l| cy == l.cell_y + l.cell_h / 2);

        if let Some(leaf) = label_overlay {
            // Truncate the label to fit the leaf's cell width.
            let max_chars = leaf.cell_w.saturating_sub(2).max(1);
            let label: String = leaf.label.chars().take(max_chars).collect();
            // Compose the line by overwriting cells [center_x - len/2, center_x + len/2)
            // with the label. The simplest robust approach: emit the rendered line
            // up to that cell range, then a foreground escape + the label + reset.
            let center_cell_x = leaf.cell_x + leaf.cell_w / 2;
            let label_start_cell = center_cell_x.saturating_sub(label.chars().count() / 2);
            let label_end_cell = label_start_cell + label.chars().count();
            // Walk the rendered line cell-by-cell; replace cells in [label_start, label_end)
            // with white text on top of the existing background.
            out.push_str(&overlay_label(
                line,
                label_start_cell,
                label_end_cell,
                &label,
            ));
        } else {
            out.push_str(line);
        }
        out.push('\n');
    }

    // Legend.
    let legend: Vec<String> = layout
        .leaves
        .iter()
        .map(|l| {
            let is_focal = story.focal.as_deref() == Some(&l.label);
            let marker = if is_focal { "■" } else { "·" };
            format!(" {marker} {} ({:.0})", l.label, l.value)
        })
        .collect();
    out.push('\n');
    out.push_str(&legend.join("  "));
    out.push('\n');

    if let Some(t) = story.takeaway {
        out.push('\n');
        out.push_str(&t);
        out.push('\n');
    }

    Ok(out)
}

/// Overwrite cells [start, end) of a rendered halfblocks line with a label.
/// The line contains ANSI escapes; we walk it character-by-character, treating
/// printable chars as one cell each and skipping over escape sequences.
/// At cell `start`, we emit a white foreground; at cell `end`, we emit a reset.
fn overlay_label(line: &str, start_cell: usize, end_cell: usize, label: &str) -> String {
    let mut out = String::with_capacity(line.len() + 32);
    let mut chars = line.chars().peekable();
    let mut cell = 0usize;
    let mut label_iter = label.chars();

    while let Some(c) = chars.next() {
        if c == '\x1b' && chars.peek() == Some(&'[') {
            // Pass through escape sequence verbatim.
            out.push(c);
            for c2 in chars.by_ref() {
                out.push(c2);
                if c2.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            // A printable cell.
            if cell >= start_cell && cell < end_cell {
                if cell == start_cell {
                    out.push_str("\x1b[38;2;240;246;252m");
                }
                if let Some(lc) = label_iter.next() {
                    out.push(lc);
                } else {
                    out.push(' ');
                }
                if cell + 1 == end_cell {
                    out.push_str("\x1b[0m");
                }
            } else {
                out.push(c);
            }
            cell += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use tplot_core::input::parse_csv_str;

    #[test]
    fn renders_treemap_with_focal_largest_leaf() {
        // 4 holdings; AAPL is the largest. Story-pass picks it as focal.
        let csv = "asset,weight\nAAPL,40\nMSFT,25\nGOOG,20\nMETA,15\n";
        let df = parse_csv_str(csv).unwrap();
        let opts = TreeOptions {
            x: "asset".into(),
            y: "weight".into(),
            focus: None,
            annotate: None,
            neutral: false,
            no_takeaway: false,
            graphics: "none".into(),
            width: Some(80),
            height: 16,
            palette_name: "signature".into(),
        };
        let out = render_treemap(&df, &opts).unwrap();
        // Burnt-orange focal escape should appear (AAPL is focal).
        assert!(out.contains("\x1b[38;2;238;123;61m"), "missing focal color");
        // The legend should list every leaf.
        assert!(out.contains("AAPL"));
        assert!(out.contains("META"));
        // Takeaway names AAPL.
        assert!(out.contains("AAPL"));
    }
}
