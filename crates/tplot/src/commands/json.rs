use crate::commands::{
    HistogramOptions, LineOptions, RenderOptions, ScatterOptions, render_bar, render_histogram,
    render_line, render_scatter,
};
use anyhow::{Result, anyhow};
use tplot_core::input::parse_json_str;
use tplot_protocol::{Axis, BarOrientation, ChartKind};

pub fn render_from_json(json: &str, canvas_w: usize, canvas_h: usize) -> Result<String> {
    let parsed = parse_json_str(json).map_err(|e| anyhow!(e.to_string()))?;
    let spec = parsed
        .spec
        .ok_or_else(|| anyhow!("missing chart spec in JSON"))?;
    match spec.kind {
        ChartKind::Bar {
            orientation: BarOrientation::Horizontal,
        } => {
            let x = match spec.x {
                Axis::Column(c) => c,
                _ => {
                    return Err(anyhow!(
                        "inline x axis not supported in v1; pass via `data` columns"
                    ));
                }
            };
            let y = match spec.y {
                Axis::Column(c) => c,
                _ => return Err(anyhow!("inline y axis not supported in v1")),
            };
            let opts = RenderOptions {
                x,
                y,
                group: spec.group,
                vertical: false,
                focus: match spec.story.focus {
                    tplot_protocol::FocusMode::Series(s) => Some(s),
                    _ => None,
                },
                annotate: spec.story.annotation,
                neutral: !spec.story.enabled,
                no_takeaway: !spec.story.takeaway,
                width: Some(canvas_w),
                height: canvas_h,
                palette_name: "signature".into(),
            };
            render_bar(&parsed.dataframe, &opts)
        }
        ChartKind::Bar {
            orientation: BarOrientation::Vertical,
        } => {
            let x = match spec.x {
                Axis::Column(c) => c,
                _ => {
                    return Err(anyhow!(
                        "inline x axis not supported in v1; pass via `data` columns"
                    ));
                }
            };
            let y = match spec.y {
                Axis::Column(c) => c,
                _ => return Err(anyhow!("inline y axis not supported in v1")),
            };
            let opts = RenderOptions {
                x,
                y,
                group: spec.group,
                vertical: true,
                focus: match spec.story.focus {
                    tplot_protocol::FocusMode::Series(s) => Some(s),
                    _ => None,
                },
                annotate: spec.story.annotation,
                neutral: !spec.story.enabled,
                no_takeaway: !spec.story.takeaway,
                width: Some(canvas_w),
                height: canvas_h,
                palette_name: "signature".into(),
            };
            render_bar(&parsed.dataframe, &opts)
        }
        ChartKind::Line => {
            let x = match spec.x {
                Axis::Column(c) => c,
                _ => return Err(anyhow!("inline x axis not supported in v1")),
            };
            let y = match spec.y {
                Axis::Column(c) => c,
                _ => return Err(anyhow!("inline y axis not supported in v1")),
            };
            let opts = LineOptions {
                x,
                y,
                group: spec.group,
                focus: match spec.story.focus {
                    tplot_protocol::FocusMode::Series(s) => Some(s),
                    _ => None,
                },
                annotate: spec.story.annotation,
                neutral: !spec.story.enabled,
                no_takeaway: !spec.story.takeaway,
                width: Some(canvas_w),
                height: canvas_h,
                palette_name: "signature".into(),
            };
            render_line(&parsed.dataframe, &opts)
        }
        ChartKind::Scatter => {
            let x = match spec.x {
                Axis::Column(c) => c,
                _ => return Err(anyhow!("inline x axis not supported in v1")),
            };
            let y = match spec.y {
                Axis::Column(c) => c,
                _ => return Err(anyhow!("inline y axis not supported in v1")),
            };
            let opts = ScatterOptions {
                x,
                y,
                group: spec.group,
                focus: match spec.story.focus {
                    tplot_protocol::FocusMode::Series(s) => Some(s),
                    _ => None,
                },
                annotate: spec.story.annotation,
                neutral: !spec.story.enabled,
                no_takeaway: !spec.story.takeaway,
                width: Some(canvas_w),
                height: canvas_h,
                palette_name: "signature".into(),
            };
            render_scatter(&parsed.dataframe, &opts)
        }
        ChartKind::Histogram { bins } => {
            let x = match spec.x {
                Axis::Column(c) => c,
                _ => {
                    return Err(anyhow!(
                        "inline x axis not supported in v1; pass via `data` columns"
                    ));
                }
            };
            let opts = HistogramOptions {
                x,
                bins,
                focus: match spec.story.focus {
                    tplot_protocol::FocusMode::Series(s) => Some(s),
                    _ => None,
                },
                annotate: spec.story.annotation,
                neutral: !spec.story.enabled,
                no_takeaway: !spec.story.takeaway,
                width: Some(canvas_w),
                height: canvas_h,
                palette_name: "signature".into(),
            };
            render_histogram(&parsed.dataframe, &opts)
        }
        ChartKind::Sparkline => Err(anyhow!("sparkline JSON dispatch not wired yet")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_bar_from_json_protocol() {
        let j = r#"{
            "kind": "bar",
            "orientation": "horizontal",
            "x": {"Column": "region"},
            "y": {"Column": "revenue"},
            "data": {
                "region":  ["NA","EMEA","LATAM","APAC","AU"],
                "revenue": [179, 193, 78, 97, 53]
            }
        }"#;
        let out = render_from_json(j, 80, 12).unwrap();
        assert!(out.contains("EMEA"));
        assert!(out.contains("\x1b[38;2;238;123;61m"));
    }
}
