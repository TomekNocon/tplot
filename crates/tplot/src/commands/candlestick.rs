use crate::pipeline::{detected_terminal_size, require_minimum_width, resolve_graphics};
use anyhow::{Result, anyhow};
use tplot_core::PixelBuffer;
use tplot_core::dataframe::DataFrame;
use tplot_core::layout::layout_candlestick;
use tplot_core::rasterize::rasterize_candlestick;
use tplot_protocol::{Capabilities, GraphicsProtocol, RgbColor};
use tplot_render::render_halfblocks;

const UP_COLOR: RgbColor = RgbColor {
    r: 0x3f,
    g: 0xb9,
    b: 0x50,
};
const DOWN_COLOR: RgbColor = RgbColor {
    r: 0xf8,
    g: 0x51,
    b: 0x49,
};

#[derive(Debug, Clone)]
pub struct CandleOptions {
    pub x: String,
    pub open: String,
    pub high: String,
    pub low: String,
    pub close: String,
    pub graphics: String,
    pub width: Option<usize>,
    pub height: usize,
}

pub fn render_candlestick(df: &DataFrame, opts: &CandleOptions) -> Result<String> {
    let (canvas_w, _) = require_minimum_width(opts.width)?;
    let canvas_h = opts.height;

    let layout = layout_candlestick(
        df,
        &opts.x,
        &opts.open,
        &opts.high,
        &opts.low,
        &opts.close,
        canvas_w,
        canvas_h,
    )
    .map_err(|e| anyhow!(e.to_string()))?;

    let mut buf = PixelBuffer::new(layout.plot_box.pixel_width, layout.plot_box.pixel_height);
    rasterize_candlestick(&layout, UP_COLOR, DOWN_COLOR, &mut buf);

    let caps = Capabilities::from_vars(|name| std::env::var(name).ok());
    let protocol = resolve_graphics(&opts.graphics, caps);

    if protocol != GraphicsProtocol::None {
        let mut out = Vec::new();
        out.extend(tplot_render::graphics::render_graphics(&buf, protocol, 6));
        out.push(b'\n');
        return Ok(String::from_utf8_lossy(&out).to_string());
    }

    // Text path: y-axis labels + chart + x-axis labels.
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

    // X-axis labels — one per candle, centered.
    let mut x_axis = String::new();
    x_axis.push_str(&" ".repeat(layout.left_margin));
    let leading = layout
        .candles
        .first()
        .map(|c| c.pixel_x.saturating_sub(layout.bar_cell_width / 2))
        .unwrap_or(0);
    x_axis.push_str(&" ".repeat(leading));
    let group_w = layout.bar_cell_width + layout.gap_cell_width;
    for (idx, c) in layout.candles.iter().enumerate() {
        let trimmed: String = c.label.chars().take(group_w).collect();
        let label_chars = trimmed.chars().count();
        let pad_left = layout.bar_cell_width.saturating_sub(label_chars) / 2;
        let pad_right = layout.bar_cell_width.saturating_sub(label_chars + pad_left);
        x_axis.push_str(&" ".repeat(pad_left));
        x_axis.push_str(&trimmed);
        x_axis.push_str(&" ".repeat(pad_right));
        if idx + 1 < layout.candles.len() {
            x_axis.push_str(&" ".repeat(layout.gap_cell_width));
        }
    }
    out.push_str(&x_axis);
    out.push('\n');

    Ok(out)
}

#[allow(dead_code)]
fn _typecheck(_: usize) -> Option<usize> {
    detected_terminal_size(None).0.into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tplot_core::input::parse_csv_str;

    #[test]
    fn renders_candlestick_with_up_and_down_days() {
        let csv = "date,o,h,l,c\nd1,100,112,98,110\nd2,110,113,99,105\nd3,105,118,104,115\nd4,115,117,113,115\n";
        let df = parse_csv_str(csv).unwrap();
        let opts = CandleOptions {
            x: "date".into(),
            open: "o".into(),
            high: "h".into(),
            low: "l".into(),
            close: "c".into(),
            graphics: "none".into(),
            width: Some(80),
            height: 16,
        };
        let out = render_candlestick(&df, &opts).unwrap();
        // d1 (up: 100→110): green color escape
        assert!(
            out.contains("\x1b[38;2;63;185;80m"),
            "missing green up-color"
        );
        // d2 (down: 110→105): red color escape
        assert!(
            out.contains("\x1b[38;2;248;81;73m"),
            "missing red down-color"
        );
        // Day labels appear under the chart.
        assert!(out.contains("d1"));
    }
}
