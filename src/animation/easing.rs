use serde::{Deserialize, Serialize};
use std::f32::consts::PI;

/// Advanced easing functions for smooth animations
/// Supports traditional CSS easing plus physics-based functions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EasingFunction {
    Linear,
    Ease,
    EaseIn,
    EaseOut,
    EaseInOut,
    EaseInSine,
    EaseOutSine,
    EaseInOutSine,
    EaseInQuad,
    EaseOutQuad,
    EaseInOutQuad,
    EaseInCubic,
    EaseOutCubic,
    EaseInOutCubic,
    EaseInQuart,
    EaseOutQuart,
    EaseInOutQuart,
    EaseInQuint,
    EaseOutQuint,
    EaseInOutQuint,
    EaseInExpo,
    EaseOutExpo,
    EaseInOutExpo,
    EaseInCirc,
    EaseOutCirc,
    EaseInOutCirc,
    EaseInBack,
    EaseOutBack,
    EaseInOutBack,
    EaseInElastic,
    EaseOutElastic,
    EaseInOutElastic,
    EaseInBounce,
    EaseOutBounce,
    EaseInOutBounce,
    // Physics-based easing
    Spring { stiffness: f32, damping: f32 },
    // Custom bezier curve
    CubicBezier { x1: f32, y1: f32, x2: f32, y2: f32 },
}

impl EasingFunction {
    /// Create easing function from string name
    pub fn from_name(name: &str) -> Self {
        match name.to_lowercase().as_str() {
            "linear" => EasingFunction::Linear,
            "ease" => EasingFunction::Ease,
            "easein" | "ease-in" => EasingFunction::EaseIn,
            "easeout" | "ease-out" => EasingFunction::EaseOut,
            "easeinout" | "ease-in-out" => EasingFunction::EaseInOut,
            "easeinsine" | "ease-in-sine" => EasingFunction::EaseInSine,
            "easeoutsine" | "ease-out-sine" => EasingFunction::EaseOutSine,
            "easeinoutsine" | "ease-in-out-sine" => EasingFunction::EaseInOutSine,
            "easeinquad" | "ease-in-quad" => EasingFunction::EaseInQuad,
            "easeoutquad" | "ease-out-quad" => EasingFunction::EaseOutQuad,
            "easeinoutquad" | "ease-in-out-quad" => EasingFunction::EaseInOutQuad,
            "easeincubic" | "ease-in-cubic" => EasingFunction::EaseInCubic,
            "easeoutcubic" | "ease-out-cubic" => EasingFunction::EaseOutCubic,
            "easeinoutcubic" | "ease-in-out-cubic" => EasingFunction::EaseInOutCubic,
            "easeinquart" | "ease-in-quart" => EasingFunction::EaseInQuart,
            "easeoutquart" | "ease-out-quart" => EasingFunction::EaseOutQuart,
            "easeinoutquart" | "ease-in-out-quart" => EasingFunction::EaseInOutQuart,
            "easeinquint" | "ease-in-quint" => EasingFunction::EaseInQuint,
            "easeoutquint" | "ease-out-quint" => EasingFunction::EaseOutQuint,
            "easeinoutquint" | "ease-in-out-quint" => EasingFunction::EaseInOutQuint,
            "easeinexpo" | "ease-in-expo" => EasingFunction::EaseInExpo,
            "easeoutexpo" | "ease-out-expo" => EasingFunction::EaseOutExpo,
            "easeinoutexpo" | "ease-in-out-expo" => EasingFunction::EaseInOutExpo,
            "easeincirc" | "ease-in-circ" => EasingFunction::EaseInCirc,
            "easeoutcirc" | "ease-out-circ" => EasingFunction::EaseOutCirc,
            "easeinoutcirc" | "ease-in-out-circ" => EasingFunction::EaseInOutCirc,
            "easeinback" | "ease-in-back" => EasingFunction::EaseInBack,
            "easeoutback" | "ease-out-back" => EasingFunction::EaseOutBack,
            "easeinoutback" | "ease-in-out-back" => EasingFunction::EaseInOutBack,
            "easeinelastic" | "ease-in-elastic" => EasingFunction::EaseInElastic,
            "easeoutelastic" | "ease-out-elastic" => EasingFunction::EaseOutElastic,
            "easeinoutelastic" | "ease-in-out-elastic" => EasingFunction::EaseInOutElastic,
            "easeinbounce" | "ease-in-bounce" => EasingFunction::EaseInBounce,
            "easeoutbounce" | "ease-out-bounce" => EasingFunction::EaseOutBounce,
            "easeinoutbounce" | "ease-in-out-bounce" => EasingFunction::EaseInOutBounce,
            "bounce" => EasingFunction::EaseOutBounce,
            "elastic" => EasingFunction::EaseOutElastic,
            "spring" => EasingFunction::Spring {
                stiffness: 300.0,
                damping: 30.0,
            },
            _ => {
                // Try to parse as cubic-bezier
                if name.starts_with("cubic-bezier(") && name.ends_with(")") {
                    if let Some(bezier) = Self::parse_cubic_bezier(name) {
                        return bezier;
                    }
                }
                EasingFunction::EaseInOut // Default fallback
            }
        }
    }

    /// Parse cubic-bezier(x1,y1,x2,y2) format
    fn parse_cubic_bezier(input: &str) -> Option<Self> {
        let inner = input.strip_prefix("cubic-bezier(")?.strip_suffix(")")?;
        let parts: Vec<&str> = inner.split(',').map(|s| s.trim()).collect();

        if parts.len() == 4 {
            let x1 = parts[0].parse::<f32>().ok()?;
            let y1 = parts[1].parse::<f32>().ok()?;
            let x2 = parts[2].parse::<f32>().ok()?;
            let y2 = parts[3].parse::<f32>().ok()?;

            Some(EasingFunction::CubicBezier { x1, y1, x2, y2 })
        } else {
            None
        }
    }

    /// Apply easing function to progress value (0.0 to 1.0)
    pub fn apply(&self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);

        match self {
            EasingFunction::Linear => t,
            EasingFunction::Ease => self.cubic_bezier(t, 0.25, 0.1, 0.25, 1.0),
            EasingFunction::EaseIn => self.cubic_bezier(t, 0.42, 0.0, 1.0, 1.0),
            EasingFunction::EaseOut => self.cubic_bezier(t, 0.0, 0.0, 0.58, 1.0),
            EasingFunction::EaseInOut => self.cubic_bezier(t, 0.42, 0.0, 0.58, 1.0),

            // Sine
            EasingFunction::EaseInSine => 1.0 - (t * PI / 2.0).cos(),
            EasingFunction::EaseOutSine => (t * PI / 2.0).sin(),
            EasingFunction::EaseInOutSine => -(PI * t).cos() / 2.0 + 0.5,

            // Quadratic
            EasingFunction::EaseInQuad => t * t,
            EasingFunction::EaseOutQuad => 1.0 - (1.0 - t) * (1.0 - t),
            EasingFunction::EaseInOutQuad => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    1.0 - 2.0 * (1.0 - t) * (1.0 - t)
                }
            }

            // Cubic
            EasingFunction::EaseInCubic => t * t * t,
            EasingFunction::EaseOutCubic => 1.0 - (1.0 - t).powi(3),
            EasingFunction::EaseInOutCubic => {
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
                }
            }

            // Quartic
            EasingFunction::EaseInQuart => t * t * t * t,
            EasingFunction::EaseOutQuart => 1.0 - (1.0 - t).powi(4),
            EasingFunction::EaseInOutQuart => {
                if t < 0.5 {
                    8.0 * t * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(4) / 2.0
                }
            }

            // Quintic
            EasingFunction::EaseInQuint => t * t * t * t * t,
            EasingFunction::EaseOutQuint => 1.0 - (1.0 - t).powi(5),
            EasingFunction::EaseInOutQuint => {
                if t < 0.5 {
                    16.0 * t * t * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(5) / 2.0
                }
            }

            // Exponential
            EasingFunction::EaseInExpo => {
                if t == 0.0 {
                    0.0
                } else {
                    2.0_f32.powf(10.0 * (t - 1.0))
                }
            }
            EasingFunction::EaseOutExpo => {
                if t == 1.0 {
                    1.0
                } else {
                    1.0 - 2.0_f32.powf(-10.0 * t)
                }
            }
            EasingFunction::EaseInOutExpo => {
                if t == 0.0 {
                    0.0
                } else if t == 1.0 {
                    1.0
                } else if t < 0.5 {
                    2.0_f32.powf(20.0 * t - 10.0) / 2.0
                } else {
                    (2.0 - 2.0_f32.powf(-20.0 * t + 10.0)) / 2.0
                }
            }

            // Circular
            EasingFunction::EaseInCirc => 1.0 - (1.0 - t * t).sqrt(),
            EasingFunction::EaseOutCirc => (1.0 - (t - 1.0) * (t - 1.0)).sqrt(),
            EasingFunction::EaseInOutCirc => {
                if t < 0.5 {
                    (1.0 - (1.0 - (2.0 * t).powi(2)).sqrt()) / 2.0
                } else {
                    ((1.0 - (-2.0 * t + 2.0).powi(2)).sqrt() + 1.0) / 2.0
                }
            }

            // Back
            EasingFunction::EaseInBack => {
                let c1 = 1.70158;
                let c3 = c1 + 1.0;
                c3 * t * t * t - c1 * t * t
            }
            EasingFunction::EaseOutBack => {
                let c1 = 1.70158;
                let c3 = c1 + 1.0;
                1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2)
            }
            EasingFunction::EaseInOutBack => {
                let c1 = 1.70158;
                let c2 = c1 * 1.525;
                if t < 0.5 {
                    ((2.0 * t).powi(2) * ((c2 + 1.0) * 2.0 * t - c2)) / 2.0
                } else {
                    ((2.0 * t - 2.0).powi(2) * ((c2 + 1.0) * (t * 2.0 - 2.0) + c2) + 2.0) / 2.0
                }
            }

            // Elastic
            EasingFunction::EaseInElastic => {
                if t == 0.0 {
                    0.0
                } else if t == 1.0 {
                    1.0
                } else {
                    -2.0_f32.powf(10.0 * (t - 1.0)) * ((t * 10.0 - 10.75) * 2.0 * PI / 3.0).sin()
                }
            }
            EasingFunction::EaseOutElastic => {
                if t == 0.0 {
                    0.0
                } else if t == 1.0 {
                    1.0
                } else {
                    2.0_f32.powf(-10.0 * t) * ((t * 10.0 - 0.75) * 2.0 * PI / 3.0).sin() + 1.0
                }
            }
            EasingFunction::EaseInOutElastic => {
                if t == 0.0 {
                    0.0
                } else if t == 1.0 {
                    1.0
                } else if t < 0.5 {
                    -(2.0_f32.powf(20.0 * t - 10.0) * ((20.0 * t - 11.125) * 2.0 * PI / 4.5).sin())
                        / 2.0
                } else {
                    2.0_f32.powf(-20.0 * t + 10.0) * ((20.0 * t - 11.125) * 2.0 * PI / 4.5).sin()
                        / 2.0
                        + 1.0
                }
            }

            // Bounce
            EasingFunction::EaseInBounce => 1.0 - self.bounce_out(1.0 - t),
            EasingFunction::EaseOutBounce => self.bounce_out(t),
            EasingFunction::EaseInOutBounce => {
                if t < 0.5 {
                    (1.0 - self.bounce_out(1.0 - 2.0 * t)) / 2.0
                } else {
                    (1.0 + self.bounce_out(2.0 * t - 1.0)) / 2.0
                }
            }

            // Physics-based Spring
            EasingFunction::Spring { stiffness, damping } => {
                self.spring_easing(t, *stiffness, *damping)
            }

            // Custom cubic bezier
            EasingFunction::CubicBezier { x1, y1, x2, y2 } => {
                self.cubic_bezier(t, *x1, *y1, *x2, *y2)
            }
        }
    }

    /// Bounce out implementation
    fn bounce_out(&self, t: f32) -> f32 {
        let n1 = 7.5625;
        let d1 = 2.75;

        if t < 1.0 / d1 {
            n1 * t * t
        } else if t < 2.0 / d1 {
            n1 * (t - 1.5 / d1) * (t - 1.5 / d1) + 0.75
        } else if t < 2.5 / d1 {
            n1 * (t - 2.25 / d1) * (t - 2.25 / d1) + 0.9375
        } else {
            n1 * (t - 2.625 / d1) * (t - 2.625 / d1) + 0.984375
        }
    }

    /// Spring physics easing with damped oscillation
    fn spring_easing(&self, t: f32, stiffness: f32, damping: f32) -> f32 {
        let omega = (stiffness / 1.0).sqrt(); // mass = 1.0 for simplicity
        let zeta = damping / (2.0 * omega);

        if zeta < 1.0 {
            // Underdamped spring
            let omega_d = omega * (1.0 - zeta * zeta).sqrt();
            let envelope = (-zeta * omega * t).exp();
            1.0 - envelope * (omega_d * t + zeta * omega / omega_d * (omega_d * t).sin()).cos()
        } else if zeta == 1.0 {
            // Critically damped spring
            1.0 - (-omega * t).exp() * (1.0 + omega * t)
        } else {
            // Overdamped spring
            let r1 = -omega * (zeta + (zeta * zeta - 1.0).sqrt());
            let r2 = -omega * (zeta - (zeta * zeta - 1.0).sqrt());
            let c1 = -r2 / (r1 - r2);
            let c2 = r1 / (r1 - r2);
            1.0 - (c1 * (r1 * t).exp() + c2 * (r2 * t).exp())
        }
    }

    /// Cubic bezier implementation for custom curves
    fn cubic_bezier(&self, t: f32, _x1: f32, y1: f32, _x2: f32, y2: f32) -> f32 {
        // Simplified cubic bezier - in production would use Newton-Raphson method
        // This is a basic approximation for demonstration
        let u = 1.0 - t;
        let tt = t * t;
        let uu = u * u;
        let uuu = uu * u;
        let ttt = tt * t;

        // Cubic bezier formula: B(t) = (1-t)³P₀ + 3(1-t)²tP₁ + 3(1-t)t²P₂ + t³P₃
        // Where P₀ = (0,0), P₁ = (x1,y1), P₂ = (x2,y2), P₃ = (1,1)
        uuu * 0.0 + 3.0 * uu * t * y1 + 3.0 * u * tt * y2 + ttt * 1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linear_easing() {
        let easing = EasingFunction::Linear;
        assert_eq!(easing.apply(0.0), 0.0);
        assert_eq!(easing.apply(0.5), 0.5);
        assert_eq!(easing.apply(1.0), 1.0);
    }

    #[test]
    fn test_ease_in_out() {
        let easing = EasingFunction::EaseInOut;
        assert_eq!(easing.apply(0.0), 0.0);
        assert!(easing.apply(0.5) > 0.4 && easing.apply(0.5) < 0.6);
        assert_eq!(easing.apply(1.0), 1.0);
    }

    #[test]
    fn test_bounce() {
        let easing = EasingFunction::EaseOutBounce;
        let result = easing.apply(0.8);
        assert!(result > 0.8 && result <= 1.0);
    }

    #[test]
    fn test_from_name() {
        let easing = EasingFunction::from_name("ease-in-out");
        match easing {
            EasingFunction::EaseInOut => {}
            _ => panic!("Failed to parse ease-in-out"),
        }
    }

    #[test]
    fn test_cubic_bezier_parsing() {
        let easing = EasingFunction::from_name("cubic-bezier(0.25, 0.1, 0.25, 1.0)");
        match easing {
            EasingFunction::CubicBezier { x1, y1, x2, y2 } => {
                assert_eq!(x1, 0.25);
                assert_eq!(y1, 0.1);
                assert_eq!(x2, 0.25);
                assert_eq!(y2, 1.0);
            }
            _ => panic!("Failed to parse cubic-bezier"),
        }
    }
}
