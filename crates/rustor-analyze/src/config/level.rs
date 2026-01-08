//! PHPStan level definitions

/// PHPStan analysis levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    /// Level 0: Basic errors (undefined functions/classes)
    Level0 = 0,
    /// Level 1: Undefined variables
    Level1 = 1,
    /// Level 2: Unknown methods on expressions
    Level2 = 2,
    /// Level 3: Return type checks
    Level3 = 3,
    /// Level 4: Dead code
    Level4 = 4,
    /// Level 5: Argument type checks
    Level5 = 5,
    /// Level 6: Missing typehints
    Level6 = 6,
    /// Level 7: Union type checks
    Level7 = 7,
    /// Level 8: Nullable checks
    Level8 = 8,
    /// Level 9: Mixed type checks (strictest)
    Level9 = 9,
}

impl Level {
    pub fn from_u8(n: u8) -> Self {
        match n {
            0 => Level::Level0,
            1 => Level::Level1,
            2 => Level::Level2,
            3 => Level::Level3,
            4 => Level::Level4,
            5 => Level::Level5,
            6 => Level::Level6,
            7 => Level::Level7,
            8 => Level::Level8,
            _ => Level::Level9,
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "0" => Some(Level::Level0),
            "1" => Some(Level::Level1),
            "2" => Some(Level::Level2),
            "3" => Some(Level::Level3),
            "4" => Some(Level::Level4),
            "5" => Some(Level::Level5),
            "6" => Some(Level::Level6),
            "7" => Some(Level::Level7),
            "8" => Some(Level::Level8),
            "9" | "max" => Some(Level::Level9),
            _ => None,
        }
    }

    pub fn as_u8(&self) -> u8 {
        *self as u8
    }
}

impl Default for Level {
    fn default() -> Self {
        Level::Level0
    }
}

impl std::fmt::Display for Level {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_u8())
    }
}

/// Description of what each level checks
pub fn level_description(level: Level) -> &'static str {
    match level {
        Level::Level0 => "Basic errors: undefined functions and classes",
        Level::Level1 => "Level 0 + undefined variables",
        Level::Level2 => "Level 1 + unknown methods on expressions, property access",
        Level::Level3 => "Level 2 + return type verification",
        Level::Level4 => "Level 3 + dead code detection",
        Level::Level5 => "Level 4 + argument type checking",
        Level::Level6 => "Level 5 + missing typehints",
        Level::Level7 => "Level 6 + union type strictness",
        Level::Level8 => "Level 7 + nullable strictness",
        Level::Level9 => "Strictest level: mixed type checks",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_level_from_str() {
        assert_eq!(Level::from_str("0"), Some(Level::Level0));
        assert_eq!(Level::from_str("5"), Some(Level::Level5));
        assert_eq!(Level::from_str("max"), Some(Level::Level9));
        assert_eq!(Level::from_str("invalid"), None);
    }

    #[test]
    fn test_level_ordering() {
        assert!(Level::Level0 < Level::Level5);
        assert!(Level::Level5 < Level::Level9);
    }
}
