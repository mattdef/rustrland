use serde::{Deserialize, Serialize};

/// Animatable property values with interpolation support
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum PropertyValue {
    Pixels(i32),
    Percentage(f32),
    Float(f32),
    Color(Color),
    Transform(Transform),
    Vector2D { x: f32, y: f32 },
    Vector3D { x: f32, y: f32, z: f32 },
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Transform {
    pub translate_x: f32,
    pub translate_y: f32,
    pub scale_x: f32,
    pub scale_y: f32,
    pub rotation: f32, // in degrees
    pub skew_x: f32,
    pub skew_y: f32,
}

/// Defines which properties of a window can be animated
#[derive(Debug, Clone)]
pub struct AnimationProperty {
    pub name: String,
    pub current_value: PropertyValue,
    pub target_value: PropertyValue,
    pub velocity: f32, // For physics-based animations
}

impl PropertyValue {
    /// Interpolate between two property values
    pub fn interpolate(&self, target: &PropertyValue, progress: f32) -> PropertyValue {
        match (self, target) {
            (PropertyValue::Pixels(from), PropertyValue::Pixels(to)) => {
                PropertyValue::Pixels(Self::lerp_i32(*from, *to, progress))
            }
            (PropertyValue::Percentage(from), PropertyValue::Percentage(to)) => {
                PropertyValue::Percentage(Self::lerp_f32(*from, *to, progress))
            }
            (PropertyValue::Float(from), PropertyValue::Float(to)) => {
                PropertyValue::Float(Self::lerp_f32(*from, *to, progress))
            }
            (PropertyValue::Color(from), PropertyValue::Color(to)) => {
                PropertyValue::Color(from.interpolate(to, progress))
            }
            (PropertyValue::Transform(from), PropertyValue::Transform(to)) => {
                PropertyValue::Transform(from.interpolate(to, progress))
            }
            (PropertyValue::Vector2D { x: x1, y: y1 }, PropertyValue::Vector2D { x: x2, y: y2 }) => {
                PropertyValue::Vector2D {
                    x: Self::lerp_f32(*x1, *x2, progress),
                    y: Self::lerp_f32(*y1, *y2, progress),
                }
            }
            (PropertyValue::Vector3D { x: x1, y: y1, z: z1 }, PropertyValue::Vector3D { x: x2, y: y2, z: z2 }) => {
                PropertyValue::Vector3D {
                    x: Self::lerp_f32(*x1, *x2, progress),
                    y: Self::lerp_f32(*y1, *y2, progress),
                    z: Self::lerp_f32(*z1, *z2, progress),
                }
            }
            // Type mismatches - return current value
            _ => self.clone(),
        }
    }
    
    /// Linear interpolation for f32
    fn lerp_f32(from: f32, to: f32, progress: f32) -> f32 {
        from + (to - from) * progress
    }
    
    /// Linear interpolation for i32
    fn lerp_i32(from: i32, to: i32, progress: f32) -> i32 {
        (from as f32 + (to - from) as f32 * progress) as i32
    }
    
    /// Get value as pixels (for positions/sizes)
    pub fn as_pixels(&self) -> i32 {
        match self {
            PropertyValue::Pixels(val) => *val,
            PropertyValue::Percentage(val) => (*val * 1920.0) as i32, // Assume 1920 screen width
            PropertyValue::Float(val) => *val as i32,
            _ => 0,
        }
    }
    
    /// Get value as float
    pub fn as_float(&self) -> f32 {
        match self {
            PropertyValue::Pixels(val) => *val as f32,
            PropertyValue::Percentage(val) => *val,
            PropertyValue::Float(val) => *val,
            _ => 0.0,
        }
    }
    
    /// Convert to CSS transform string for Hyprland
    pub fn to_css_transform(&self) -> String {
        match self {
            PropertyValue::Transform(transform) => transform.to_css_string(),
            PropertyValue::Vector2D { x, y } => format!("translate({}px, {}px)", x, y),
            PropertyValue::Vector3D { x, y, z } => format!("translate3d({}px, {}px, {}px)", x, y, z),
            PropertyValue::Float(scale) => format!("scale({})", scale),
            _ => String::new(),
        }
    }
    
    /// Create from string value with unit parsing
    pub fn from_string(value: &str) -> Result<PropertyValue, String> {
        let value = value.trim();
        
        // Parse pixels
        if value.ends_with("px") {
            let num_str = value.trim_end_matches("px");
            if let Ok(pixels) = num_str.parse::<i32>() {
                return Ok(PropertyValue::Pixels(pixels));
            }
        }
        
        // Parse percentage
        if value.ends_with('%') {
            let num_str = value.trim_end_matches('%');
            if let Ok(percent) = num_str.parse::<f32>() {
                return Ok(PropertyValue::Percentage(percent));
            }
        }
        
        // Parse RGB color
        if value.starts_with("rgb(") && value.ends_with(')') {
            return Color::from_rgb_string(value)
                .map(PropertyValue::Color)
                .map_err(|e| format!("Invalid RGB color: {}", e));
        }
        
        // Parse RGBA color
        if value.starts_with("rgba(") && value.ends_with(')') {
            return Color::from_rgba_string(value)
                .map(PropertyValue::Color)
                .map_err(|e| format!("Invalid RGBA color: {}", e));
        }
        
        // Parse hex color
        if value.starts_with('#') {
            return Color::from_hex_string(value)
                .map(PropertyValue::Color)
                .map_err(|e| format!("Invalid hex color: {}", e));
        }
        
        // Parse float
        if let Ok(float_val) = value.parse::<f32>() {
            return Ok(PropertyValue::Float(float_val));
        }
        
        Err(format!("Cannot parse property value: {}", value))
    }
}

impl Color {
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self {
            r: r.clamp(0.0, 1.0),
            g: g.clamp(0.0, 1.0),
            b: b.clamp(0.0, 1.0),
            a: a.clamp(0.0, 1.0),
        }
    }
    
    /// Interpolate between colors
    pub fn interpolate(&self, target: &Color, progress: f32) -> Color {
        Color {
            r: self.r + (target.r - self.r) * progress,
            g: self.g + (target.g - self.g) * progress,
            b: self.b + (target.b - self.b) * progress,
            a: self.a + (target.a - self.a) * progress,
        }
    }
    
    /// Parse RGB color from string like "rgb(255, 128, 0)"
    pub fn from_rgb_string(rgb_str: &str) -> Result<Color, String> {
        let inner = rgb_str.trim_start_matches("rgb(").trim_end_matches(')');
        let parts: Vec<&str> = inner.split(',').map(|s| s.trim()).collect();
        
        if parts.len() != 3 {
            return Err("RGB color must have 3 components".to_string());
        }
        
        let r = parts[0].parse::<u8>().map_err(|_| "Invalid red component")?;
        let g = parts[1].parse::<u8>().map_err(|_| "Invalid green component")?;
        let b = parts[2].parse::<u8>().map_err(|_| "Invalid blue component")?;
        
        Ok(Color::new(
            r as f32 / 255.0,
            g as f32 / 255.0,
            b as f32 / 255.0,
            1.0,
        ))
    }
    
    /// Parse RGBA color from string like "rgba(255, 128, 0, 0.5)"
    pub fn from_rgba_string(rgba_str: &str) -> Result<Color, String> {
        let inner = rgba_str.trim_start_matches("rgba(").trim_end_matches(')');
        let parts: Vec<&str> = inner.split(',').map(|s| s.trim()).collect();
        
        if parts.len() != 4 {
            return Err("RGBA color must have 4 components".to_string());
        }
        
        let r = parts[0].parse::<u8>().map_err(|_| "Invalid red component")?;
        let g = parts[1].parse::<u8>().map_err(|_| "Invalid green component")?;
        let b = parts[2].parse::<u8>().map_err(|_| "Invalid blue component")?;
        let a = parts[3].parse::<f32>().map_err(|_| "Invalid alpha component")?;
        
        Ok(Color::new(
            r as f32 / 255.0,
            g as f32 / 255.0,
            b as f32 / 255.0,
            a,
        ))
    }
    
    /// Parse hex color from string like "#FF8000" or "#FF8000AA"
    pub fn from_hex_string(hex_str: &str) -> Result<Color, String> {
        let hex = hex_str.trim_start_matches('#');
        
        match hex.len() {
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).map_err(|_| "Invalid hex color")?;
                let g = u8::from_str_radix(&hex[2..4], 16).map_err(|_| "Invalid hex color")?;
                let b = u8::from_str_radix(&hex[4..6], 16).map_err(|_| "Invalid hex color")?;
                
                Ok(Color::new(
                    r as f32 / 255.0,
                    g as f32 / 255.0,
                    b as f32 / 255.0,
                    1.0,
                ))
            }
            8 => {
                let r = u8::from_str_radix(&hex[0..2], 16).map_err(|_| "Invalid hex color")?;
                let g = u8::from_str_radix(&hex[2..4], 16).map_err(|_| "Invalid hex color")?;
                let b = u8::from_str_radix(&hex[4..6], 16).map_err(|_| "Invalid hex color")?;
                let a = u8::from_str_radix(&hex[6..8], 16).map_err(|_| "Invalid hex color")?;
                
                Ok(Color::new(
                    r as f32 / 255.0,
                    g as f32 / 255.0,
                    b as f32 / 255.0,
                    a as f32 / 255.0,
                ))
            }
            _ => Err("Hex color must be 6 or 8 characters".to_string()),
        }
    }
    
    /// Convert to hex string
    pub fn to_hex_string(&self) -> String {
        format!(
            "#{:02X}{:02X}{:02X}{:02X}",
            (self.r * 255.0) as u8,
            (self.g * 255.0) as u8,
            (self.b * 255.0) as u8,
            (self.a * 255.0) as u8
        )
    }
}

impl Transform {
    pub fn new() -> Self {
        Self {
            translate_x: 0.0,
            translate_y: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
            rotation: 0.0,
            skew_x: 0.0,
            skew_y: 0.0,
        }
    }
    
    /// Interpolate between transforms
    pub fn interpolate(&self, target: &Transform, progress: f32) -> Transform {
        Transform {
            translate_x: self.translate_x + (target.translate_x - self.translate_x) * progress,
            translate_y: self.translate_y + (target.translate_y - self.translate_y) * progress,
            scale_x: self.scale_x + (target.scale_x - self.scale_x) * progress,
            scale_y: self.scale_y + (target.scale_y - self.scale_y) * progress,
            rotation: self.rotation + (target.rotation - self.rotation) * progress,
            skew_x: self.skew_x + (target.skew_x - self.skew_x) * progress,
            skew_y: self.skew_y + (target.skew_y - self.skew_y) * progress,
        }
    }
    
    /// Convert to CSS transform string
    pub fn to_css_string(&self) -> String {
        let mut transforms = Vec::new();
        
        if self.translate_x != 0.0 || self.translate_y != 0.0 {
            transforms.push(format!("translate({}px, {}px)", self.translate_x, self.translate_y));
        }
        
        if self.scale_x != 1.0 || self.scale_y != 1.0 {
            transforms.push(format!("scale({}, {})", self.scale_x, self.scale_y));
        }
        
        if self.rotation != 0.0 {
            transforms.push(format!("rotate({}deg)", self.rotation));
        }
        
        if self.skew_x != 0.0 {
            transforms.push(format!("skewX({}deg)", self.skew_x));
        }
        
        if self.skew_y != 0.0 {
            transforms.push(format!("skewY({}deg)", self.skew_y));
        }
        
        transforms.join(" ")
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::new()
    }
}

impl AnimationProperty {
    pub fn new(name: String, current: PropertyValue, target: PropertyValue) -> Self {
        Self {
            name,
            current_value: current,
            target_value: target,
            velocity: 0.0,
        }
    }
    
    /// Update the current value based on progress
    pub fn update(&mut self, progress: f32) {
        self.current_value = self.current_value.interpolate(&self.target_value, progress);
    }
    
    /// Get the difference between current and target values (for physics)
    pub fn get_delta(&self) -> f32 {
        match (&self.current_value, &self.target_value) {
            (PropertyValue::Pixels(current), PropertyValue::Pixels(target)) => {
                (*target - *current) as f32
            }
            (PropertyValue::Float(current), PropertyValue::Float(target)) => {
                *target - *current
            }
            (PropertyValue::Percentage(current), PropertyValue::Percentage(target)) => {
                *target - *current
            }
            _ => 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_property_interpolation() {
        let from = PropertyValue::Pixels(100);
        let to = PropertyValue::Pixels(200);
        
        let result = from.interpolate(&to, 0.5);
        assert_eq!(result, PropertyValue::Pixels(150));
    }
    
    #[test]
    fn test_color_interpolation() {
        let red = Color::new(1.0, 0.0, 0.0, 1.0);
        let blue = Color::new(0.0, 0.0, 1.0, 1.0);
        
        let purple = red.interpolate(&blue, 0.5);
        assert_eq!(purple.r, 0.5);
        assert_eq!(purple.g, 0.0);
        assert_eq!(purple.b, 0.5);
        assert_eq!(purple.a, 1.0);
    }
    
    #[test]
    fn test_color_parsing() {
        let color = Color::from_rgb_string("rgb(255, 128, 0)").unwrap();
        assert_eq!(color.r, 1.0);
        assert_eq!(color.g, 128.0 / 255.0);
        assert_eq!(color.b, 0.0);
        assert_eq!(color.a, 1.0);
    }
    
    #[test]
    fn test_hex_color_parsing() {
        let color = Color::from_hex_string("#FF8000").unwrap();
        assert_eq!(color.r, 1.0);
        assert_eq!(color.g, 128.0 / 255.0);
        assert_eq!(color.b, 0.0);
        assert_eq!(color.a, 1.0);
    }
    
    #[test]
    fn test_transform_interpolation() {
        let from = Transform {
            translate_x: 0.0,
            translate_y: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
            rotation: 0.0,
            skew_x: 0.0,
            skew_y: 0.0,
        };
        
        let to = Transform {
            translate_x: 100.0,
            translate_y: 200.0,
            scale_x: 2.0,
            scale_y: 2.0,
            rotation: 90.0,
            skew_x: 0.0,
            skew_y: 0.0,
        };
        
        let result = from.interpolate(&to, 0.5);
        assert_eq!(result.translate_x, 50.0);
        assert_eq!(result.translate_y, 100.0);
        assert_eq!(result.scale_x, 1.5);
        assert_eq!(result.scale_y, 1.5);
        assert_eq!(result.rotation, 45.0);
    }
    
    #[test]
    fn test_property_value_parsing() {
        assert_eq!(PropertyValue::from_string("100px").unwrap(), PropertyValue::Pixels(100));
        assert_eq!(PropertyValue::from_string("50%").unwrap(), PropertyValue::Percentage(50.0));
        assert_eq!(PropertyValue::from_string("1.5").unwrap(), PropertyValue::Float(1.5));
        
        let color = match PropertyValue::from_string("#FF0000").unwrap() {
            PropertyValue::Color(c) => c,
            _ => panic!("Expected color"),
        };
        assert_eq!(color.r, 1.0);
        assert_eq!(color.g, 0.0);
        assert_eq!(color.b, 0.0);
    }
}