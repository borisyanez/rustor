//! Trinary (three-valued) logic for type analysis
//!
//! PHPStan uses trinary logic to express certainty about type relationships:
//! - Yes: Definitely true
//! - Maybe: Possibly true (unknown)
//! - No: Definitely false

/// Three-valued logic result
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TrinaryLogic {
    /// Definitely true
    Yes,
    /// Possibly true (unknown/uncertain)
    Maybe,
    /// Definitely false
    No,
}

impl TrinaryLogic {
    /// Returns true if the result is Yes
    pub fn yes(&self) -> bool {
        matches!(self, TrinaryLogic::Yes)
    }

    /// Returns true if the result is No
    pub fn no(&self) -> bool {
        matches!(self, TrinaryLogic::No)
    }

    /// Returns true if the result is Maybe
    pub fn maybe(&self) -> bool {
        matches!(self, TrinaryLogic::Maybe)
    }

    /// Logical AND
    pub fn and(self, other: TrinaryLogic) -> TrinaryLogic {
        match (self, other) {
            (TrinaryLogic::No, _) | (_, TrinaryLogic::No) => TrinaryLogic::No,
            (TrinaryLogic::Yes, TrinaryLogic::Yes) => TrinaryLogic::Yes,
            _ => TrinaryLogic::Maybe,
        }
    }

    /// Logical OR
    pub fn or(self, other: TrinaryLogic) -> TrinaryLogic {
        match (self, other) {
            (TrinaryLogic::Yes, _) | (_, TrinaryLogic::Yes) => TrinaryLogic::Yes,
            (TrinaryLogic::No, TrinaryLogic::No) => TrinaryLogic::No,
            _ => TrinaryLogic::Maybe,
        }
    }

    /// Logical NOT
    pub fn not(self) -> TrinaryLogic {
        match self {
            TrinaryLogic::Yes => TrinaryLogic::No,
            TrinaryLogic::No => TrinaryLogic::Yes,
            TrinaryLogic::Maybe => TrinaryLogic::Maybe,
        }
    }

    /// Create from boolean
    pub fn from_bool(value: bool) -> TrinaryLogic {
        if value {
            TrinaryLogic::Yes
        } else {
            TrinaryLogic::No
        }
    }

    /// Combine multiple results with AND
    pub fn and_all(results: impl IntoIterator<Item = TrinaryLogic>) -> TrinaryLogic {
        let mut result = TrinaryLogic::Yes;
        for r in results {
            result = result.and(r);
            if result.no() {
                break;
            }
        }
        result
    }

    /// Combine multiple results with OR
    pub fn or_all(results: impl IntoIterator<Item = TrinaryLogic>) -> TrinaryLogic {
        let mut result = TrinaryLogic::No;
        for r in results {
            result = result.or(r);
            if result.yes() {
                break;
            }
        }
        result
    }
}

impl Default for TrinaryLogic {
    fn default() -> Self {
        TrinaryLogic::Maybe
    }
}

impl From<bool> for TrinaryLogic {
    fn from(value: bool) -> Self {
        TrinaryLogic::from_bool(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_and() {
        assert_eq!(TrinaryLogic::Yes.and(TrinaryLogic::Yes), TrinaryLogic::Yes);
        assert_eq!(TrinaryLogic::Yes.and(TrinaryLogic::No), TrinaryLogic::No);
        assert_eq!(TrinaryLogic::Yes.and(TrinaryLogic::Maybe), TrinaryLogic::Maybe);
        assert_eq!(TrinaryLogic::No.and(TrinaryLogic::Maybe), TrinaryLogic::No);
    }

    #[test]
    fn test_or() {
        assert_eq!(TrinaryLogic::Yes.or(TrinaryLogic::No), TrinaryLogic::Yes);
        assert_eq!(TrinaryLogic::No.or(TrinaryLogic::No), TrinaryLogic::No);
        assert_eq!(TrinaryLogic::No.or(TrinaryLogic::Maybe), TrinaryLogic::Maybe);
    }

    #[test]
    fn test_not() {
        assert_eq!(TrinaryLogic::Yes.not(), TrinaryLogic::No);
        assert_eq!(TrinaryLogic::No.not(), TrinaryLogic::Yes);
        assert_eq!(TrinaryLogic::Maybe.not(), TrinaryLogic::Maybe);
    }
}
