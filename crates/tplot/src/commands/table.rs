use crate::pipeline::require_minimum_width;
use anyhow::{Result, anyhow};
use tplot_core::dataframe::DataFrame;
use tplot_core::table::{ColumnType, TableLayoutConfig, layout_table};
use tplot_protocol::{Capabilities, Palette};

#[derive(Debug, Clone, Default)]
pub struct TableOptions {
    pub bars: Option<String>,
    pub sort: Option<String>,
    pub top: Option<usize>,
    pub focus: Option<String>, // "column=value" form
    pub annotate: Option<String>,
    pub neutral: bool,
    pub no_takeaway: bool,
    pub graphics: String, // ignored — tables are pure text
    pub width: Option<usize>,
    pub palette_name: String,
    pub rounded: bool,
}

const PAD: usize = 1;
const BAR_CELLS: usize = 10;

#[allow(clippy::needless_range_loop)]
pub fn render_table(df: &DataFrame, opts: &TableOptions) -> Result<String> {
    let palette = if opts.palette_name.is_empty() {
        Palette::Signature
    } else {
        Palette::from_name(&opts.palette_name).map_err(|e| anyhow!(e.to_string()))?
    };
    let (canvas_w, _) = require_minimum_width(opts.width)?;

    let cfg = TableLayoutConfig {
        bars: opts.bars.clone(),
        sort: opts.sort.clone(),
        top: opts.top,
        focus_text: opts.focus.as_ref().and_then(|s| {
            let (k, v) = s.split_once('=')?;
            Some((k.to_string(), v.to_string()))
        }),
    };
    let layout = layout_table(df, cfg).map_err(|e| anyhow!(e.to_string()))?;

    let caps = Capabilities::from_vars(|name| std::env::var(name).ok());
    let focal_color = palette.focal_color();
    let context_color = palette.context_color_for(caps.theme);

    // Compute per-column render widths.
    let n_cols = layout.columns.len();
    let mut col_w: Vec<usize> = (0..n_cols)
        .map(|ci| {
            let header_w = layout.columns[ci].name.chars().count();
            let values_w = layout
                .rows
                .iter()
                .map(|r| r[ci].chars().count())
                .max()
                .unwrap_or(0);
            header_w.max(values_w)
        })
        .collect();

    // Bars column gets extra width for `value space bar`.
    if let Some(bi) = layout.bars_col_idx {
        col_w[bi] += 1 + BAR_CELLS; // space + bar
    }

    // Truncate columns if total exceeds canvas_w.
    let total_render_w = total_table_width(&col_w, opts.rounded);
    if total_render_w > canvas_w {
        truncate_columns(&mut col_w, total_render_w - canvas_w, layout.bars_col_idx);
    }

    // ----- Render -----
    let mut out = String::new();
    let glyphs = if opts.rounded { ROUNDED } else { SQUARE };

    // Top border.
    out.push_str(&border_line(
        &col_w, glyphs.tl, glyphs.t_t, glyphs.tr, glyphs.h,
    ));
    out.push('\n');

    // Header row.
    out.push_str(&render_row(
        &col_w,
        &layout
            .columns
            .iter()
            .map(|c| c.name.clone())
            .collect::<Vec<_>>(),
        &layout.columns,
        layout.bars_col_idx,
        None, // no bar value on header
        glyphs.v,
        true,  // bold
        false, // not focal
        focal_color,
        context_color,
    ));
    out.push('\n');

    // Header-data separator.
    out.push_str(&border_line(
        &col_w,
        glyphs.l_t,
        glyphs.cross,
        glyphs.r_t,
        glyphs.h,
    ));
    out.push('\n');

    // Body rows.
    for (ri, row) in layout.rows.iter().enumerate() {
        let bar_value = layout.bars_values.as_ref().map(|v| {
            let max = layout
                .bars_col_idx
                .and_then(|bi| layout.columns[bi].max_numeric)
                .unwrap_or(0.0);
            (v[ri], max)
        });
        let is_focal = !opts.neutral && Some(ri) == layout.focal_row;
        out.push_str(&render_row(
            &col_w,
            row,
            &layout.columns,
            layout.bars_col_idx,
            bar_value,
            glyphs.v,
            false,
            is_focal,
            focal_color,
            context_color,
        ));
        out.push('\n');
    }

    // Bottom border.
    out.push_str(&border_line(
        &col_w, glyphs.bl, glyphs.t_b, glyphs.br, glyphs.h,
    ));
    out.push('\n');

    // Takeaway.
    if !opts.no_takeaway {
        if let Some(ann) = &opts.annotate {
            out.push('\n');
            out.push_str(ann);
            out.push('\n');
        } else if let Some(ri) = layout.focal_row
            && let Some(bi) = layout.bars_col_idx
        {
            let row = &layout.rows[ri];
            // Find a useful "name" for the row — first text column.
            let name_idx = layout
                .columns
                .iter()
                .position(|c| matches!(c.kind, ColumnType::Text))
                .unwrap_or(0);
            let name = &row[name_idx];
            let value = &row[bi];
            out.push('\n');
            out.push_str(&format!(
                "{} leads with {} {}.",
                name, value, layout.columns[bi].name
            ));
            out.push('\n');
        }
    }

    Ok(out)
}

// -------- Helpers (border glyphs and per-row rendering) --------

struct BorderGlyphs {
    tl: char,
    tr: char,
    bl: char,
    br: char, // corners
    t_t: char,
    t_b: char,
    l_t: char,
    r_t: char, // T-junctions
    cross: char,
    h: char,
    v: char,
}

const SQUARE: BorderGlyphs = BorderGlyphs {
    tl: '┌',
    tr: '┐',
    bl: '└',
    br: '┘',
    t_t: '┬',
    t_b: '┴',
    l_t: '├',
    r_t: '┤',
    cross: '┼',
    h: '─',
    v: '│',
};
const ROUNDED: BorderGlyphs = BorderGlyphs {
    tl: '╭',
    tr: '╮',
    bl: '╰',
    br: '╯',
    t_t: '┬',
    t_b: '┴',
    l_t: '├',
    r_t: '┤',
    cross: '┼',
    h: '─',
    v: '│',
};

fn total_table_width(col_w: &[usize], _rounded: bool) -> usize {
    // Each cell: 1 padding + col_w + 1 padding + 1 vertical separator.
    // Plus a leading vertical separator.
    1 + col_w.iter().map(|w| 1 + w + 1 + 1).sum::<usize>()
}

fn truncate_columns(col_w: &mut [usize], excess: usize, _exclude_idx: Option<usize>) {
    // Naive: shrink the widest column repeatedly until excess is gone.
    let mut left = excess;
    while left > 0 {
        let widest = col_w
            .iter()
            .enumerate()
            .max_by_key(|&(_, &w)| w)
            .map(|(i, _)| i);
        match widest {
            Some(i) if col_w[i] > 4 => {
                col_w[i] -= 1;
                left -= 1;
            }
            _ => break,
        }
    }
}

fn border_line(col_w: &[usize], left: char, mid: char, right: char, h: char) -> String {
    let mut s = String::new();
    s.push(left);
    for (i, &w) in col_w.iter().enumerate() {
        for _ in 0..(w + PAD * 2) {
            s.push(h);
        }
        if i + 1 < col_w.len() {
            s.push(mid);
        }
    }
    s.push(right);
    s
}

#[allow(clippy::too_many_arguments)]
fn render_row(
    col_w: &[usize],
    cells: &[String],
    columns: &[tplot_core::table::TableColumn],
    bars_col_idx: Option<usize>,
    bar_value: Option<(f64, f64)>,
    v_glyph: char,
    bold: bool,
    focal: bool,
    focal_color: tplot_protocol::RgbColor,
    context_color: tplot_protocol::RgbColor,
) -> String {
    use std::fmt::Write as _;

    // Track the active ANSI style across cells in the row so we don't emit
    // a redundant escape for every cell. The body color (focal vs context)
    // is constant across a row, so we precompute it and compare cell styles
    // via the Style enum below.
    let body_color_str: String = if focal {
        format!(
            "\x1b[38;2;{};{};{}m",
            focal_color.r, focal_color.g, focal_color.b
        )
    } else {
        format!(
            "\x1b[38;2;{};{};{}m",
            context_color.r, context_color.g, context_color.b
        )
    };
    // Tag each cell's required style by a stable id so we can compare cheaply.
    #[derive(PartialEq, Eq, Clone, Copy)]
    enum Style {
        None,
        Bold,
        Body,
    }
    let cell_style: Style = if bold { Style::Bold } else { Style::Body };
    let mut current: Style = Style::None;

    let open = |s: &mut String, want: Style, current: &mut Style| {
        if *current == want {
            return;
        }
        // Switching styles: reset first (only if something is active).
        if *current != Style::None {
            s.push_str("\x1b[0m");
        }
        match want {
            Style::None => {}
            Style::Bold => s.push_str("\x1b[1m"),
            Style::Body => s.push_str(&body_color_str),
        }
        *current = want;
    };
    let close_if_active = |s: &mut String, current: &mut Style| {
        if *current != Style::None {
            s.push_str("\x1b[0m");
            *current = Style::None;
        }
    };

    let mut s = String::new();
    s.push(v_glyph);
    for (ci, value) in cells.iter().enumerate() {
        s.push(' ');
        let cell_w = col_w[ci];

        // For the bars column on body rows, render `[value] [bar]`.
        let (text_part, bar_part) = if Some(ci) == bars_col_idx && !bold {
            if let Some((v, max)) = bar_value {
                let bar_w = BAR_CELLS.min(cell_w.saturating_sub(value.chars().count() + 1));
                let bar_str = bar_glyphs(v, max, bar_w);
                (
                    truncate(value, cell_w.saturating_sub(bar_w + 1)),
                    Some(bar_str),
                )
            } else {
                (truncate(value, cell_w), None)
            }
        } else {
            (truncate(value, cell_w), None)
        };

        // Determine alignment.
        let align_right = matches!(columns[ci].kind, ColumnType::Number);
        let align_center = matches!(columns[ci].kind, ColumnType::Boolean);

        let text_w = text_part.chars().count();
        let bar_w = bar_part
            .as_ref()
            .map(|b| b.chars().count() + 1)
            .unwrap_or(0);
        let used = text_w + bar_w;
        let pad = cell_w.saturating_sub(used);

        if align_right {
            // Whitespace pad first — close any active style so spaces aren't
            // styled (cheap reset, then we re-open when needed).
            close_if_active(&mut s, &mut current);
            for _ in 0..pad {
                s.push(' ');
            }
            open(&mut s, cell_style, &mut current);
            let _ = write!(s, "{text_part}");
            if let Some(bar) = &bar_part {
                close_if_active(&mut s, &mut current);
                s.push(' ');
                open(&mut s, cell_style, &mut current);
                let _ = write!(s, "{bar}");
            }
        } else if align_center {
            close_if_active(&mut s, &mut current);
            let lp = pad / 2;
            let rp = pad - lp;
            for _ in 0..lp {
                s.push(' ');
            }
            open(&mut s, cell_style, &mut current);
            let _ = write!(s, "{text_part}");
            close_if_active(&mut s, &mut current);
            for _ in 0..rp {
                s.push(' ');
            }
        } else {
            open(&mut s, cell_style, &mut current);
            let _ = write!(s, "{text_part}");
            close_if_active(&mut s, &mut current);
            for _ in 0..pad {
                s.push(' ');
            }
        }
        // Vertical separator: ensure plain output (no styled glyphs).
        close_if_active(&mut s, &mut current);
        s.push(' ');
        s.push(v_glyph);
    }
    // End-of-row reset, in case the last cell left styling open.
    close_if_active(&mut s, &mut current);
    s
}

fn truncate(s: &str, max: usize) -> String {
    let n = s.chars().count();
    if n <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

fn bar_glyphs(value: f64, max: f64, width: usize) -> String {
    if max <= 0.0 || width == 0 {
        return String::new();
    }
    let frac = (value / max).clamp(0.0, 1.0);
    let total_eighths = (frac * (width * 8) as f64).round() as usize;
    let full = total_eighths / 8;
    let partial_eighths = total_eighths % 8;
    let mut s = String::with_capacity(width);
    for _ in 0..full.min(width) {
        s.push('█');
    }
    if full < width && partial_eighths > 0 {
        let g = match partial_eighths {
            1 => '▏',
            2 => '▎',
            3 => '▍',
            4 => '▌',
            5 => '▋',
            6 => '▊',
            7 => '▉',
            _ => unreachable!(),
        };
        s.push(g);
    }
    while s.chars().count() < width {
        s.push(' ');
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use tplot_core::input::parse_csv_str;

    #[test]
    fn renders_basic_table_with_headers() {
        let csv = "region,revenue\nNA,179\nEMEA,193\nLATAM,78\n";
        let df = parse_csv_str(csv).unwrap();
        let opts = TableOptions::default();
        let out = render_table(&df, &opts).unwrap();
        // Headers appear:
        assert!(out.contains("region"));
        assert!(out.contains("revenue"));
        // Box drawing chars present:
        assert!(out.contains('┌') || out.contains('╭'));
        assert!(out.contains('│'));
        // All rows present:
        assert!(out.contains("NA"));
        assert!(out.contains("EMEA"));
        assert!(out.contains("LATAM"));
    }

    #[test]
    fn renders_focal_row_in_burnt_orange() {
        // 5-row dataset where the max (193) is well above the median (97),
        // giving trust ~ 1.99 > 1.5 to clear the focal-row gate.
        let csv = "region,revenue\nNA,179\nEMEA,193\nLATAM,78\nAPAC,97\nAU,53\n";
        let df = parse_csv_str(csv).unwrap();
        let opts = TableOptions {
            bars: Some("revenue".into()),
            ..Default::default()
        };
        let out = render_table(&df, &opts).unwrap();
        // EMEA is the max → focal → burnt orange escape appears.
        assert!(out.contains("\x1b[38;2;238;123;61m"), "missing focal color");
    }

    #[test]
    fn renders_inline_bar_for_bars_column() {
        let csv = "region,revenue\nNA,100\nEMEA,200\nLATAM,50\n";
        let df = parse_csv_str(csv).unwrap();
        let opts = TableOptions {
            bars: Some("revenue".into()),
            ..Default::default()
        };
        let out = render_table(&df, &opts).unwrap();
        // Some bar glyph (▁..█) should appear in the rendered output.
        let has_bar = out.chars().any(|c| {
            matches!(
                c,
                '\u{2581}'
                    | '\u{2582}'
                    | '\u{2583}'
                    | '\u{2584}'
                    | '\u{2585}'
                    | '\u{2586}'
                    | '\u{2587}'
                    | '\u{2588}'
            )
        });
        assert!(has_bar, "expected inline bar glyphs");
    }

    #[test]
    fn sort_column_orders_rows_descending() {
        let csv = "region,revenue\nNA,100\nEMEA,200\nLATAM,50\n";
        let df = parse_csv_str(csv).unwrap();
        let opts = TableOptions {
            sort: Some("revenue".into()),
            ..Default::default()
        };
        let out = render_table(&df, &opts).unwrap();
        // EMEA should appear before NA.
        let emea_pos = out.find("EMEA").unwrap();
        let na_pos = out.find("NA").unwrap();
        assert!(emea_pos < na_pos, "EMEA should be sorted first");
    }
}
