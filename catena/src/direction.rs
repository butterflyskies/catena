//! Validated direction with inverse tracking.
//!
//! Invariant 1.8: Direction construction validates syntax — non-empty, printable
//! ASCII only, no whitespace, ≤64 characters. Every direction has an inverse.
//! Standard cardinal directions provide built-in inverse pairs; custom directions
//! require an explicit inverse at construction.

use std::fmt;
use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};

/// Maximum length of a direction string.
const MAX_LEN: usize = 64;

/// A validated direction with a known inverse.
///
/// Standard directions (north, south, east, west, up, down) have built-in
/// inverses. Custom directions carry their inverse explicitly.
///
/// Identity is determined by `name` alone — two `Direction` values with the
/// same name are equal regardless of their inverse field. This keeps
/// `HashMap<Direction, _>` lookups consistent when the same direction name
/// appears with different inverse metadata.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Direction {
    /// The direction name (e.g. "north", "portal-in").
    name: String,
    /// The inverse direction name (e.g. "south", "portal-out").
    inverse: String,
}

impl PartialEq for Direction {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Eq for Direction {}

impl Hash for Direction {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
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
    /// A direction and its inverse are identical.
    SelfInverse,
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
            Self::SelfInverse => {
                write!(f, "a direction cannot be its own inverse")
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
    /// Create a standard direction with a well-known inverse.
    ///
    /// Recognized names: `north`, `south`, `east`, `west`, `up`, `down`.
    /// Returns `None` if the name is not a standard direction.
    pub fn standard(name: &str) -> Option<Self> {
        let (name, inverse) = match name {
            "north" => ("north", "south"),
            "south" => ("south", "north"),
            "east" => ("east", "west"),
            "west" => ("west", "east"),
            "up" => ("up", "down"),
            "down" => ("down", "up"),
            _ => return None,
        };
        Some(Self {
            name: name.to_owned(),
            inverse: inverse.to_owned(),
        })
    }

    /// Create a custom direction with an explicit inverse.
    ///
    /// Both `name` and `inverse` are validated. They must not be identical —
    /// a direction cannot be its own inverse (self-referencing *exits* are valid
    /// per invariant 1.3, but a direction must still have a distinct reverse).
    pub fn custom(
        name: impl Into<String>,
        inverse: impl Into<String>,
    ) -> Result<Self, DirectionError> {
        let name = name.into();
        let inverse = inverse.into();

        validate_direction_str(&name)?;
        validate_direction_str(&inverse)?;

        if name == inverse {
            return Err(DirectionError::SelfInverse);
        }

        Ok(Self { name, inverse })
    }

    /// The direction name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The inverse direction name.
    pub fn inverse_name(&self) -> &str {
        &self.inverse
    }

    /// Produce the inverse `Direction` (swapping name and inverse).
    pub fn inverse(&self) -> Self {
        Self {
            name: self.inverse.clone(),
            inverse: self.name.clone(),
        }
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

    // --- standard directions ---

    #[test]
    fn standard_north() {
        let dir = Direction::standard("north").unwrap();
        assert_eq!(dir.name(), "north");
        assert_eq!(dir.inverse_name(), "south");
    }

    #[test]
    fn standard_round_trip() {
        let north = Direction::standard("north").unwrap();
        let south = north.inverse();
        assert_eq!(south.name(), "south");
        assert_eq!(south.inverse_name(), "north");
        let north_again = south.inverse();
        assert_eq!(north_again, north);
    }

    #[test]
    fn all_standard_directions_exist() {
        for name in &["north", "south", "east", "west", "up", "down"] {
            assert!(
                Direction::standard(name).is_some(),
                "missing standard direction: {name}"
            );
        }
    }

    #[test]
    fn nonstandard_returns_none() {
        assert!(Direction::standard("northwest").is_none());
        assert!(Direction::standard("portal").is_none());
    }

    // --- custom directions ---

    #[test]
    fn custom_direction() {
        let dir = Direction::custom("portal-in", "portal-out").unwrap();
        assert_eq!(dir.name(), "portal-in");
        assert_eq!(dir.inverse_name(), "portal-out");
    }

    #[test]
    fn custom_inverse_round_trip() {
        let dir = Direction::custom("enter", "exit").unwrap();
        let inv = dir.inverse();
        assert_eq!(inv.name(), "exit");
        assert_eq!(inv.inverse_name(), "enter");
        assert_eq!(inv.inverse(), dir);
    }

    // --- validation ---

    #[test]
    fn reject_empty_name() {
        assert_eq!(Direction::custom("", "back"), Err(DirectionError::Empty));
    }

    #[test]
    fn reject_empty_inverse() {
        assert_eq!(Direction::custom("forward", ""), Err(DirectionError::Empty));
    }

    #[test]
    fn reject_too_long() {
        let long = "a".repeat(MAX_LEN + 1);
        assert!(matches!(
            Direction::custom(&long, "back"),
            Err(DirectionError::TooLong(_))
        ));
    }

    #[test]
    fn reject_whitespace() {
        assert!(matches!(
            Direction::custom("north west", "south east"),
            Err(DirectionError::Whitespace { .. })
        ));
    }

    #[test]
    fn reject_non_ascii() {
        assert!(matches!(
            Direction::custom("n\u{00f6}rd", "s\u{00fc}d"),
            Err(DirectionError::InvalidChar { .. })
        ));
    }

    #[test]
    fn reject_self_inverse() {
        assert_eq!(
            Direction::custom("loop", "loop"),
            Err(DirectionError::SelfInverse)
        );
    }

    #[test]
    fn exactly_max_length_ok() {
        let name = "a".repeat(MAX_LEN);
        let inv = "b".repeat(MAX_LEN);
        assert!(Direction::custom(&name, &inv).is_ok());
    }

    // --- trait impls ---

    #[test]
    fn display() {
        let dir = Direction::standard("east").unwrap();
        assert_eq!(format!("{dir}"), "east");
    }

    #[test]
    fn hash_and_eq() {
        use std::collections::HashSet;
        let a = Direction::standard("north").unwrap();
        let b = Direction::standard("north").unwrap();
        let c = Direction::standard("south").unwrap();
        let mut set = HashSet::new();
        set.insert(a.clone());
        assert!(set.contains(&b));
        assert!(!set.contains(&c));
    }

    #[test]
    fn eq_and_hash_ignore_inverse() {
        use std::collections::HashMap;
        // Two directions with the same name but different inverse fields
        let a = Direction::custom("portal", "portal-out").unwrap();
        let b = Direction::custom("portal", "portal-back").unwrap();

        // They compare equal
        assert_eq!(a, b);

        // They hash to the same bucket — inserting both into a map yields one entry
        let mut map = HashMap::new();
        map.insert(a.clone(), 1);
        map.insert(b.clone(), 2);
        assert_eq!(map.len(), 1);
        assert_eq!(map[&a], 2);
    }
}
