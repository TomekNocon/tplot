use crate::pipeline::{require_minimum_width, resolve_graphics};
use anyhow::{Result, anyhow};
use std::collections::HashMap;
use tplot_core::PixelBuffer;
use tplot_core::dataframe::{DataFrame, Series};
use tplot_core::layout::layout_sankey;
use tplot_core::rasterize::rasterize_sankey;
use tplot_core::sankey_graph::{RawEdge, build_graph};
use tplot_protocol::{Capabilities, GraphicsProtocol, Palette, RgbColor};
use tplot_render::render_halfblocks;
use tplot_story::{FocalChoice, SeriesPoint, pick_focal};

#[derive(Debug, Clone)]
pub struct SankeyOptions {
    pub source: String,
    pub target: String,
    pub value: String,
    pub focus: Option<String>,
    pub annotate: Option<String>,
    pub neutral: bool,
    pub no_takeaway: bool,
    pub graphics: String,
    pub width: Option<usize>,
    pub height: usize,
    pub palette_name: String,
}

pub fn render_sankey(df: &DataFrame, opts: &SankeyOptions) -> Result<String> {
    let palette = Palette::from_name(&opts.palette_name).map_err(|e| anyhow!(e.to_string()))?;
    let (canvas_w, _) = require_minimum_width(opts.width)?;
    let canvas_h = opts.height;

    // Build edges from the DataFrame.
    let sources: Vec<String> = match df
        .column(&opts.source)
        .map_err(|e| anyhow!(e.to_string()))?
        .series()
    {
        Series::Strings(v) => v.clone(),
        Series::Numbers(v) => v.iter().map(|n| format!("{n}")).collect(),
    };
    let targets: Vec<String> = match df
        .column(&opts.target)
        .map_err(|e| anyhow!(e.to_string()))?
        .series()
    {
        Series::Strings(v) => v.clone(),
        Series::Numbers(v) => v.iter().map(|n| format!("{n}")).collect(),
    };
    let values: Vec<f64> = match df
        .column(&opts.value)
        .map_err(|e| anyhow!(e.to_string()))?
        .series()
    {
        Series::Numbers(v) => v.clone(),
        Series::Strings(_) => return Err(anyhow!("value column `{}` must be numeric", opts.value)),
    };
    if sources.len() != targets.len() || sources.len() != values.len() {
        return Err(anyhow!("source / target / value columns differ in length"));
    }
    let raws: Vec<RawEdge> = sources
        .into_iter()
        .zip(targets)
        .zip(values)
        .map(|((s, t), v)| RawEdge {
            source: s,
            target: t,
            value: v,
        })
        .collect();

    let graph = build_graph(&raws).map_err(|e| anyhow!(e.to_string()))?;
    let layout = layout_sankey(&graph, canvas_w, canvas_h).map_err(|e| anyhow!(e.to_string()))?;

    // Story-pass: focal = biggest single edge by value (reuse `pick_focal`).
    let series_points: Vec<SeriesPoint> = layout
        .edges
        .iter()
        .map(|e| SeriesPoint {
            key: e.label.clone(),
            value: e.value,
        })
        .collect();
    let caps = Capabilities::from_vars(|name| std::env::var(name).ok());

    let focal_label: Option<String> = if opts.neutral {
        None
    } else if let Some(f) = &opts.focus {
        // User-specified focus: try to find an edge with this label.
        layout
            .edges
            .iter()
            .find(|e| &e.label == f)
            .map(|_| f.clone())
    } else {
        match pick_focal(&series_points).choice {
            FocalChoice::Series(s) => Some(s),
            FocalChoice::None => None,
        }
    };

    // Build edge palette: focal in focal color, others in context.
    let focal_color = palette.focal_color();
    let context_color = palette.context_color_for(caps.theme);
    let mut edge_palette: HashMap<String, RgbColor> = HashMap::new();
    for e in &layout.edges {
        let color = if Some(&e.label) == focal_label.as_ref() {
            focal_color
        } else {
            context_color
        };
        edge_palette.insert(e.label.clone(), color);
    }
    // Node color: a slightly darker shade so they read against the edge bands.
    let node_color = palette.dim_context_color_for(caps.theme);

    // Rasterize.
    let mut buf = PixelBuffer::new(layout.plot_box.pixel_width, layout.plot_box.pixel_height);
    rasterize_sankey(
        &layout,
        focal_label.as_deref(),
        &edge_palette,
        node_color,
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
    let mut out = body;

    // Legend: per-layer summary of nodes ("layer 0: a (100), b (50)").
    out.push('\n');
    let layered = layout.nodes.iter().fold(
        Vec::<Vec<&tplot_core::layout::SankeyNodePos>>::new(),
        |mut acc, n| {
            while acc.len() <= n.layer {
                acc.push(Vec::new());
            }
            acc[n.layer].push(n);
            acc
        },
    );
    for (li, layer) in layered.iter().enumerate() {
        let pieces: Vec<String> = layer
            .iter()
            .map(|n| format!("{} ({:.0})", n.name, n.value))
            .collect();
        let _ = li;
        out.push(' ');
        out.push_str(&pieces.join("  →  "));
        out.push('\n');
    }

    // Takeaway.
    if !opts.no_takeaway {
        out.push('\n');
        let takeaway = if let Some(custom) = &opts.annotate {
            custom.clone()
        } else if let Some(label) = &focal_label {
            let edge = layout.edges.iter().find(|e| &e.label == label);
            if let Some(edge) = edge {
                let total: f64 = layout.edges.iter().map(|e| e.value).sum();
                let pct = (edge.value / total * 100.0).round() as i64;
                format!(
                    " {} carries the largest flow — {} ({pct}% of total).",
                    edge.label,
                    format_number(edge.value)
                )
            } else {
                "".into()
            }
        } else {
            " No single flow dominates — values are distributed across edges.".to_string()
        };
        if !takeaway.is_empty() {
            out.push_str(&takeaway);
            out.push('\n');
        }
    }

    Ok(out)
}

fn format_number(v: f64) -> String {
    if v >= 1_000_000.0 {
        format!("{:.1}M", v / 1_000_000.0)
    } else if v >= 1_000.0 {
        format!("{:.1}k", v / 1_000.0)
    } else {
        format!("{:.0}", v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tplot_core::input::parse_csv_str;

    #[test]
    fn renders_sankey_with_focal_largest_edge() {
        // landing → signup is the largest edge — focal in burnt orange.
        let csv = "src,tgt,flow\n\
            landing,signup,7000\n\
            landing,bounce,3000\n\
            signup,verified,6000\n\
            signup,abandoned,1000\n";
        let df = parse_csv_str(csv).unwrap();
        let opts = SankeyOptions {
            source: "src".into(),
            target: "tgt".into(),
            value: "flow".into(),
            focus: None,
            annotate: None,
            neutral: false,
            no_takeaway: false,
            graphics: "none".into(),
            width: Some(80),
            height: 16,
            palette_name: "signature".into(),
        };
        let out = render_sankey(&df, &opts).unwrap();
        // The biggest edge (landing → signup, 7000) should be focal in burnt orange.
        assert!(out.contains("\x1b[38;2;238;123;61m"), "missing focal color");
        // Node names should appear in the legend.
        assert!(out.contains("landing"));
        assert!(out.contains("signup"));
        assert!(out.contains("verified"));
    }
}
