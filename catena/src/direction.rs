//! Validated direction name.
//!
//! Invariant 1.8: Direction construction validates syntax — non-empty, printable
//! ASCII only, no whitespace, ≤64 characters. Direction is a validated string;
//! inverse relationships are not part of the type. Bidirectional exit consistency
//! is the mudlib's responsibility (see invariant 1.2).

use std::fmt;

use serde::{Deserialize, Serialize};

/// A validated direction name.
///
/// Direction is a validated string newtype — it carries no inverse metadata.
/// Bidirectional exits are created by the mudlib calling `Exits::set()` twice
/// (once for each direction). The engine does not enforce or assume inverse
/// relationships between directions.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(into = "String", try_from = "String")]
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
                write!(f, "direction is {len} chars, maximum is {}", Direction::MAX_LEN)
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
    if s.len() > Direction::MAX_LEN {
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
    /// Maximum length of a direction string.
    pub const MAX_LEN: usize = 64;

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

impl From<Direction> for String {
    fn from(d: Direction) -> Self {
        d.name
    }
}

impl TryFrom<String> for Direction {
    type Error = DirectionError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

impl TryFrom<&str> for Direction {
    type Error = DirectionError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::new(s)
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

    fn north() -> Direction {
        Direction::new("north").unwrap()
    }

    fn south() -> Direction {
        Direction::new("south").unwrap()
    }

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
        let long = "a".repeat(Direction::MAX_LEN + 1);
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
        let name = "a".repeat(Direction::MAX_LEN);
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
        let mut set = HashSet::new();
        set.insert(north());
        assert!(set.contains(&north()));
        assert!(!set.contains(&south()));
    }

    #[test]
    fn derived_eq_is_structural() {
        assert_eq!(north(), north());
        assert_ne!(north(), south());
    }

    // --- serde ---

    #[test]
    fn serde_round_trip() {
        let dir = Direction::new("north").unwrap();
        let json = serde_json::to_string(&dir).unwrap();
        let back: Direction = serde_json::from_str(&json).unwrap();
        assert_eq!(dir, back);
    }

    #[test]
    fn serde_rejects_invalid() {
        // Whitespace in name — must fail validation on deserialize.
        let json = r#""north west""#;
        let err = serde_json::from_str::<Direction>(json);
        assert!(err.is_err());
    }

    #[test]
    fn serde_rejects_empty() {
        let json = r#""""#;
        let err = serde_json::from_str::<Direction>(json);
        assert!(err.is_err());
    }

    // --- TryFrom ---

    #[test]
    fn try_from_string() {
        let dir: Direction = String::from("north").try_into().unwrap();
        assert_eq!(dir.name(), "north");
    }

    #[test]
    fn try_from_str_ref() {
        let dir: Direction = "north".try_into().unwrap();
        assert_eq!(dir.name(), "north");
    }

    // --- proptest ---

    mod prop {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn arbitrary_string_never_panics(s in ".*") {
                let _ = Direction::new(&s);
            }

            #[test]
            fn valid_directions_survive_round_trip(name in "[a-z][a-z0-9_-]{0,20}") {
                let dir = Direction::new(&name).unwrap();
                assert_eq!(dir.name(), name.as_str());
            }

            #[test]
            fn empty_always_rejected(s in proptest::string::string_regex("").unwrap()) {
                assert_eq!(Direction::new(s), Err(DirectionError::Empty));
            }

            #[test]
            fn too_long_rejected(s in "[a-z]{65,128}") {
                assert!(s.len() > Direction::MAX_LEN);
                assert!(matches!(Direction::new(s), Err(DirectionError::TooLong(_))));
            }

            #[test]
            fn whitespace_rejected(
                pre in "[a-z]{1,4}",
                ws in "[ \\t\\n\\r]",
                post in "[a-z]{1,4}",
            ) {
                let raw = format!("{pre}{ws}{post}");
                assert!(matches!(
                    Direction::new(raw),
                    Err(DirectionError::Whitespace { .. })
                ));
            }
        }
    }
}
