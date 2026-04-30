use crate::story::StoryConfig;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ChartKind {
    Bar {
        orientation: BarOrientation,
    },
    /// Histogram of a single numeric column. Bin count is auto-computed
    /// when None (Sturges' rule); explicit override via Some(N).
    Histogram {
        #[serde(default)]
        bins: Option<usize>,
    },
    Line,
    Scatter,
    Sparkline,
    /// 2D heatmap. Long-form input: x and y axes are categorical columns;
    /// the named `value` column is the numeric color intensity.
    Heatmap {
        value: String,
    },
    /// Vertical box plots — one column per group (the `x` column),
    /// 5-number summary of the `y` column per group.
    BoxPlot,
    /// Stacked area chart. Long-form input grouped by `group`; each group
    /// stacks on top of the previous (in first-seen order). Y is the cumulative
    /// total across all groups.
    StackedArea,
    /// OHLC candlestick chart. Names of the numeric columns for each value.
    Candlestick {
        open: String,
        high: String,
        low: String,
        close: String,
    },
    /// Flat treemap — each row is one leaf rectangle. Value column drives area.
    Treemap,
    /// Violin plot — KDE-based distribution shape per category.
    Violin,
    /// Ridgeline (joy-plot) chart — stacked KDEs per group along a categorical
    /// y-axis, each ridge a filled curve over a shared x-axis.
    Ridgeline,
    /// Sankey diagram — flow between named nodes. Each row in the data is
    /// one edge: `(source, target, value)`.
    Sankey {
        source: String,
        target: String,
        value: String,
    },
    /// Pretty-printed table — auto-typed columns, optional inline value bars
    /// for a designated column, sortable, top-N filter, focal-row highlighting.
    Table {
        #[serde(default)]
        bars: Option<String>, // column name to render an inline bar for
        #[serde(default)]
        sort: Option<String>, // column name to sort by (descending)
        #[serde(default)]
        top: Option<usize>, // keep only the top N rows after sorting
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BarOrientation {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Axis {
    /// Column lookup (preferred for CSV/file inputs).
    Column(String),
    /// Inline literal data (used for the `--json` form when the caller passed
    /// values inline rather than as columns).
    Inline(Vec<serde_json::Value>),
}

// Note: `Axis::Inline` carries `serde_json::Value`, which does NOT implement
// `Eq` (floats / NaN). Hence `Axis` and the types embedding it derive only
// `PartialEq`. This is intentional — equality of a chart spec is not a
// production concern; it only shows up in tests, which use `assert_eq!`
// (which only needs `PartialEq`).

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChartSpec {
    #[serde(flatten)]
    pub kind: ChartKind,
    pub x: Axis,
    pub y: Axis,
    #[serde(default)]
    pub group: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub story: StoryConfig,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bar_spec_round_trip_json() {
        let spec = ChartSpec {
            kind: ChartKind::Bar {
                orientation: BarOrientation::Horizontal,
            },
            x: Axis::Column("quarter".into()),
            y: Axis::Column("revenue".into()),
            group: Some("region".into()),
            title: None,
            story: StoryConfig::default(),
        };
        let json = serde_json::to_string(&spec).unwrap();
        let back: ChartSpec = serde_json::from_str(&json).unwrap();
        assert_eq!(back, spec);
    }

    #[test]
    fn histogram_spec_round_trip() {
        let spec = ChartSpec {
            kind: ChartKind::Histogram { bins: Some(20) },
            x: Axis::Column("latency_ms".into()),
            y: Axis::Column("__count__".into()),
            group: None,
            title: Some("Request latency distribution".into()),
            story: StoryConfig::default(),
        };
        let json = serde_json::to_string(&spec).unwrap();
        let back: ChartSpec = serde_json::from_str(&json).unwrap();
        assert_eq!(back, spec);
    }

    #[test]
    fn line_spec_round_trip() {
        let spec = ChartSpec {
            kind: ChartKind::Line,
            x: Axis::Column("time".into()),
            y: Axis::Column("value".into()),
            group: Some("series".into()),
            title: None,
            story: StoryConfig::default(),
        };
        let json = serde_json::to_string(&spec).unwrap();
        assert_eq!(serde_json::from_str::<ChartSpec>(&json).unwrap(), spec);
    }

    #[test]
    fn sparkline_spec_round_trip() {
        let spec = ChartSpec {
            kind: ChartKind::Sparkline,
            x: Axis::Column("__index__".into()),
            y: Axis::Column("value".into()),
            group: None,
            title: None,
            story: StoryConfig::default(),
        };
        let json = serde_json::to_string(&spec).unwrap();
        assert_eq!(serde_json::from_str::<ChartSpec>(&json).unwrap(), spec);
    }

    #[test]
    fn heatmap_spec_round_trip() {
        let spec = ChartSpec {
            kind: ChartKind::Heatmap {
                value: "count".into(),
            },
            x: Axis::Column("hour".into()),
            y: Axis::Column("day".into()),
            group: None,
            title: None,
            story: StoryConfig::default(),
        };
        let json = serde_json::to_string(&spec).unwrap();
        assert_eq!(serde_json::from_str::<ChartSpec>(&json).unwrap(), spec);
    }

    #[test]
    fn boxplot_spec_round_trip() {
        let spec = ChartSpec {
            kind: ChartKind::BoxPlot,
            x: Axis::Column("endpoint".into()),
            y: Axis::Column("latency_ms".into()),
            group: None,
            title: None,
            story: StoryConfig::default(),
        };
        let json = serde_json::to_string(&spec).unwrap();
        assert_eq!(serde_json::from_str::<ChartSpec>(&json).unwrap(), spec);
    }

    #[test]
    fn stacked_area_spec_round_trip() {
        let spec = ChartSpec {
            kind: ChartKind::StackedArea,
            x: Axis::Column("month".into()),
            y: Axis::Column("revenue".into()),
            group: Some("region".into()),
            title: None,
            story: StoryConfig::default(),
        };
        let json = serde_json::to_string(&spec).unwrap();
        assert_eq!(serde_json::from_str::<ChartSpec>(&json).unwrap(), spec);
    }

    #[test]
    fn candlestick_spec_round_trip() {
        let spec = ChartSpec {
            kind: ChartKind::Candlestick {
                open: "open".into(),
                high: "high".into(),
                low: "low".into(),
                close: "close".into(),
            },
            x: Axis::Column("date".into()),
            y: Axis::Column("close".into()),
            group: None,
            title: None,
            story: StoryConfig::default(),
        };
        let json = serde_json::to_string(&spec).unwrap();
        assert_eq!(serde_json::from_str::<ChartSpec>(&json).unwrap(), spec);
    }

    #[test]
    fn scatter_spec_round_trip() {
        let spec = ChartSpec {
            kind: ChartKind::Scatter,
            x: Axis::Column("x".into()),
            y: Axis::Column("y".into()),
            group: Some("cluster".into()),
            title: None,
            story: StoryConfig::default(),
        };
        let json = serde_json::to_string(&spec).unwrap();
        assert_eq!(serde_json::from_str::<ChartSpec>(&json).unwrap(), spec);
    }

    #[test]
    fn violin_spec_round_trip() {
        let spec = ChartSpec {
            kind: ChartKind::Violin,
            x: Axis::Column("group".into()),
            y: Axis::Column("value".into()),
            group: None,
            title: None,
            story: StoryConfig::default(),
        };
        let json = serde_json::to_string(&spec).unwrap();
        assert_eq!(serde_json::from_str::<ChartSpec>(&json).unwrap(), spec);
    }

    #[test]
    fn ridgeline_spec_round_trip() {
        let spec = ChartSpec {
            kind: ChartKind::Ridgeline,
            x: Axis::Column("value".into()),
            y: Axis::Column("__count__".into()),
            group: Some("category".into()),
            title: None,
            story: StoryConfig::default(),
        };
        let json = serde_json::to_string(&spec).unwrap();
        assert_eq!(serde_json::from_str::<ChartSpec>(&json).unwrap(), spec);
    }

    #[test]
    fn sankey_spec_round_trip() {
        let spec = ChartSpec {
            kind: ChartKind::Sankey {
                source: "src".into(),
                target: "tgt".into(),
                value: "flow".into(),
            },
            x: Axis::Column("__node__".into()),
            y: Axis::Column("__flow__".into()),
            group: None,
            title: None,
            story: StoryConfig::default(),
        };
        let json = serde_json::to_string(&spec).unwrap();
        assert_eq!(serde_json::from_str::<ChartSpec>(&json).unwrap(), spec);
    }

    #[test]
    fn treemap_spec_round_trip() {
        let spec = ChartSpec {
            kind: ChartKind::Treemap,
            x: Axis::Column("category".into()),
            y: Axis::Column("value".into()),
            group: None,
            title: None,
            story: StoryConfig::default(),
        };
        let json = serde_json::to_string(&spec).unwrap();
        assert_eq!(serde_json::from_str::<ChartSpec>(&json).unwrap(), spec);
    }

    #[test]
    fn table_spec_round_trip() {
        let spec = ChartSpec {
            kind: ChartKind::Table {
                bars: Some("revenue".into()),
                sort: Some("revenue".into()),
                top: Some(10),
            },
            x: Axis::Column("__row__".into()),
            y: Axis::Column("__col__".into()),
            group: None,
            title: None,
            story: StoryConfig::default(),
        };
        let json = serde_json::to_string(&spec).unwrap();
        assert_eq!(serde_json::from_str::<ChartSpec>(&json).unwrap(), spec);
    }
}
