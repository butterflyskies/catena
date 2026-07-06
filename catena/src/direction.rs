//! Validated direction name.
//!
//! Invariant 1.8: Direction construction validates syntax — non-empty, printable
//! ASCII only, no whitespace, ≤64 characters. Direction is a validated string;
//! inverse relationships are not part of the type. Bidirectional exit consistency
//! is the mudlib's responsibility (see invariant 1.2).

use std::fmt;

use serde::{Deserialize, Serialize};

/// Maximum length of a direction string.
const MAX_LEN: usize = 64;

/// A validated direction name.
///
/// Direction is a validated string newtype — it carries no inverse metadata.
/// Bidirectional exits are created by the mudlib calling `Exits::set()` twice
/// (once for each direction). The engine does not enforce or assume inverse
/// relationships between directions.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Direction {
    name: String,
}

/// Reasons a string cannot be a valid direction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DirectionError {
    /// The input was empty.
    Empty,
    /// The input exceeds the 64-character limit.
    TooLong(usize),
    /// The input contains a non-printable-ASCII character.
    InvalidChar { offset: usize, ch: char },
    /// The input contains whitespace.
    Whitespace { offset: usize, ch: char },
}

impl fmt::Display for DirectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "direction must not be empty"),
            Self::TooLong(len) => {
                write!(f, "direction is {len} chars, maximum is {MAX_LEN}")
            }
            Self::InvalidChar { offset, ch } => {
                write!(
                    f,
                    "invalid character {ch:?} at byte offset {offset} (must be printable ASCII)"
                )
            }
            Self::Whitespace { offset, ch } => {
                write!(f, "whitespace {ch:?} at byte offset {offset}")
            }
        }
    }
}

impl std::error::Error for DirectionError {}

/// Validate that a string is acceptable as a direction name.
fn validate_direction_str(s: &str) -> Result<(), DirectionError> {
    if s.is_empty() {
        return Err(DirectionError::Empty);
    }
    if s.len() > MAX_LEN {
        return Err(DirectionError::TooLong(s.len()));
    }
    for (offset, ch) in s.char_indices() {
        if ch.is_whitespace() {
            return Err(DirectionError::Whitespace { offset, ch });
        }
        if !ch.is_ascii() || ch.is_ascii_control() {
            return Err(DirectionError::InvalidChar { offset, ch });
        }
    }
    Ok(())
}

impl Direction {
    /// Create a direction from a validated string.
    ///
    /// Accepts any non-empty, printable-ASCII, whitespace-free string up to
    /// 64 characters. Standard names like `"north"` are valid but have no
    /// special treatment — the type does not track inverses.
    pub fn new(name: impl Into<String>) -> Result<Self, DirectionError> {
        let name = name.into();
        validate_direction_str(&name)?;
        Ok(Self { name })
    }

    /// The direction name.
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl fmt::Display for Direction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- construction ---

    #[test]
    fn new_standard_names() {
        for name in &["north", "south", "east", "west", "up", "down"] {
            let dir = Direction::new(*name).unwrap();
            assert_eq!(dir.name(), *name);
        }
    }

    #[test]
    fn new_custom_name() {
        let dir = Direction::new("portal-in").unwrap();
        assert_eq!(dir.name(), "portal-in");
    }

    #[test]
    fn new_accepts_string() {
        let dir = Direction::new(String::from("cupboard")).unwrap();
        assert_eq!(dir.name(), "cupboard");
    }

    // --- validation ---

    #[test]
    fn reject_empty() {
        assert_eq!(Direction::new(""), Err(DirectionError::Empty));
    }

    #[test]
    fn reject_too_long() {
        let long = "a".repeat(MAX_LEN + 1);
        assert!(matches!(
            Direction::new(&long),
            Err(DirectionError::TooLong(_))
        ));
    }

    #[test]
    fn reject_whitespace() {
        assert!(matches!(
            Direction::new("north west"),
            Err(DirectionError::Whitespace { .. })
        ));
    }

    #[test]
    fn reject_non_ascii() {
        assert!(matches!(
            Direction::new("n\u{00f6}rd"),
            Err(DirectionError::InvalidChar { .. })
        ));
    }

    #[test]
    fn reject_control_chars() {
        assert!(matches!(
            Direction::new("north\x01"),
            Err(DirectionError::InvalidChar { .. })
        ));
    }

    #[test]
    fn exactly_max_length_ok() {
        let name = "a".repeat(MAX_LEN);
        assert!(Direction::new(&name).is_ok());
    }

    // --- trait impls ---

    #[test]
    fn display() {
        let dir = Direction::new("east").unwrap();
        assert_eq!(format!("{dir}"), "east");
    }

    #[test]
    fn hash_and_eq() {
        use std::collections::HashSet;
        let a = Direction::new("north").unwrap();
        let b = Direction::new("north").unwrap();
        let c = Direction::new("south").unwrap();
        let mut set = HashSet::new();
        set.insert(a.clone());
        assert!(set.contains(&b));
        assert!(!set.contains(&c));
    }

    #[test]
    fn derived_eq_is_structural() {
        let a = Direction::new("north").unwrap();
        let b = Direction::new("north").unwrap();
        assert_eq!(a, b);
        assert_ne!(a, Direction::new("south").unwrap());
    }
}
