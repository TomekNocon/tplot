use crate::story::StoryConfig;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ChartKind {
    Bar { orientation: BarOrientation },
    // Other variants land in subsequent plans.
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
}
