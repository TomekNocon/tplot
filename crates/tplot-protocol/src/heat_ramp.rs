use crate::color::RgbColor;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HeatRamp {
    /// Dark navy → blue → magenta → orange → red. Default.
    Inferno,
    /// Perceptually uniform purple → blue → green → yellow.
    Viridis,
    /// Blue → white → red. Diverging — best for centered-on-zero data.
    Coolwarm,
}

#[derive(Debug, thiserror::Error)]
#[error("unknown heat ramp: {0}")]
pub struct UnknownHeatRamp(pub String);

impl HeatRamp {
    pub fn from_name(name: &str) -> Result<Self, UnknownHeatRamp> {
        match name {
            "inferno" => Ok(HeatRamp::Inferno),
            "viridis" => Ok(HeatRamp::Viridis),
            "coolwarm" => Ok(HeatRamp::Coolwarm),
            other => Err(UnknownHeatRamp(other.to_string())),
        }
    }

    /// Sample the ramp at `t` ∈ [0,1] (clamped). Linearly interpolates between
    /// the surrounding stops in RGB space.
    pub fn sample(self, t: f64) -> RgbColor {
        let t = t.clamp(0.0, 1.0);
        let stops = self.stops();
        // Find the bracketing pair.
        for i in 0..stops.len() - 1 {
            let (t0, c0) = stops[i];
            let (t1, c1) = stops[i + 1];
            if t <= t1 {
                let span = (t1 - t0).max(1e-9);
                let f = ((t - t0) / span).clamp(0.0, 1.0);
                return lerp_rgb(c0, c1, f);
            }
        }
        stops[stops.len() - 1].1
    }

    fn stops(self) -> &'static [(f64, RgbColor)] {
        match self {
            HeatRamp::Inferno => &[
                (
                    0.00,
                    RgbColor {
                        r: 0x1c,
                        g: 0x23,
                        b: 0x30,
                    },
                ), // dark navy
                (
                    0.25,
                    RgbColor {
                        r: 0x2d,
                        g: 0x42,
                        b: 0x63,
                    },
                ), // deep blue
                (
                    0.50,
                    RgbColor {
                        r: 0x6e,
                        g: 0x40,
                        b: 0x90,
                    },
                ), // magenta
                (
                    0.75,
                    RgbColor {
                        r: 0xee,
                        g: 0x7b,
                        b: 0x3d,
                    },
                ), // orange (signature focal)
                (
                    1.00,
                    RgbColor {
                        r: 0xd5,
                        g: 0x4e,
                        b: 0x3a,
                    },
                ), // red
            ],
            HeatRamp::Viridis => &[
                (
                    0.00,
                    RgbColor {
                        r: 0x44,
                        g: 0x01,
                        b: 0x54,
                    },
                ), // deep purple
                (
                    0.25,
                    RgbColor {
                        r: 0x3b,
                        g: 0x52,
                        b: 0x8b,
                    },
                ), // blue
                (
                    0.50,
                    RgbColor {
                        r: 0x21,
                        g: 0x90,
                        b: 0x8c,
                    },
                ), // teal
                (
                    0.75,
                    RgbColor {
                        r: 0x5e,
                        g: 0xc9,
                        b: 0x62,
                    },
                ), // green
                (
                    1.00,
                    RgbColor {
                        r: 0xfd,
                        g: 0xe7,
                        b: 0x25,
                    },
                ), // yellow
            ],
            HeatRamp::Coolwarm => &[
                (
                    0.00,
                    RgbColor {
                        r: 0x33,
                        g: 0x66,
                        b: 0xa6,
                    },
                ), // blue
                (
                    0.50,
                    RgbColor {
                        r: 0xee,
                        g: 0xee,
                        b: 0xee,
                    },
                ), // near-white
                (
                    1.00,
                    RgbColor {
                        r: 0xc0,
                        g: 0x39,
                        b: 0x2b,
                    },
                ), // red
            ],
        }
    }
}

#[allow(clippy::derivable_impls)]
impl Default for HeatRamp {
    fn default() -> Self {
        HeatRamp::Inferno
    }
}

fn lerp_rgb(a: RgbColor, b: RgbColor, t: f64) -> RgbColor {
    let lerp = |x: u8, y: u8| -> u8 {
        (x as f64 + (y as f64 - x as f64) * t)
            .round()
            .clamp(0.0, 255.0) as u8
    };
    RgbColor {
        r: lerp(a.r, b.r),
        g: lerp(a.g, b.g),
        b: lerp(a.b, b.b),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(unused_imports)]
    use crate::RgbColor;

    #[test]
    fn inferno_at_0_is_dark_navy() {
        let c = HeatRamp::Inferno.sample(0.0);
        assert!(c.r < 50 && c.g < 60 && c.b > 30);
    }

    #[test]
    fn inferno_at_1_is_warm_red() {
        let c = HeatRamp::Inferno.sample(1.0);
        assert!(c.r > 200);
        assert!(c.g < c.r);
        assert!(c.b < c.r);
    }

    #[test]
    fn sample_is_monotonic_for_inferno() {
        // Each step should produce a *different* color along the ramp.
        let mut prev = HeatRamp::Inferno.sample(0.0);
        for i in 1..=10 {
            let cur = HeatRamp::Inferno.sample(i as f64 / 10.0);
            assert_ne!(cur, prev, "ramp stalled at step {i}");
            prev = cur;
        }
    }

    #[test]
    fn from_name_dispatches_known_ramps() {
        assert_eq!(HeatRamp::from_name("inferno").unwrap(), HeatRamp::Inferno);
        assert_eq!(HeatRamp::from_name("viridis").unwrap(), HeatRamp::Viridis);
        assert_eq!(
            HeatRamp::from_name("coolwarm").unwrap(),
            HeatRamp::Coolwarm
        );
        assert!(HeatRamp::from_name("nope").is_err());
    }

}
