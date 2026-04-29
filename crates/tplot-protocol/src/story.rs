use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum FocusMode {
    Auto,
    Series(String),
    None,
}

#[allow(clippy::derivable_impls)]
impl Default for FocusMode {
    fn default() -> Self {
        FocusMode::Auto
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoryConfig {
    /// Master switch. `--neutral` flips this off.
    pub enabled: bool,
    /// Print the auto-generated "so what" line.
    pub takeaway: bool,
    /// Focal-series resolution mode.
    pub focus: FocusMode,
    /// Optional user-supplied takeaway override.
    pub annotation: Option<String>,
}

impl Default for StoryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            takeaway: true,
            focus: FocusMode::Auto,
            annotation: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_story_on() {
        let s = StoryConfig::default();
        assert!(s.enabled);
        assert!(s.takeaway);
        assert!(matches!(s.focus, FocusMode::Auto));
    }
}
