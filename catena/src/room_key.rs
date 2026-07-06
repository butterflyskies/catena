//! Validated room identifier with namespace.
//!
//! Invariant 1.7: RoomKey construction validates syntax — non-empty, contains a
//! namespace separator (`:`), printable ASCII only, no whitespace, ≤64 characters.
//! Invalid keys are rejected at creation and never stored in the registry.

use std::fmt;

use serde::{Deserialize, Serialize};

/// The namespace separator character used in room keys.
const NAMESPACE_SEP: char = ':';

/// Maximum length of a room key in characters.
const MAX_LEN: usize = 64;

/// A validated, namespaced room identifier (e.g. `"catena:cantina/main"`).
///
/// Construction is fallible — only [`RoomKey::try_new`] and [`TryFrom<String>`]
/// produce valid instances. The inner string is not publicly accessible.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RoomKey(String);

/// Reasons a string cannot be a valid [`RoomKey`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RoomKeyError {
    /// The input was empty.
    Empty,
    /// The input exceeds the 64-character limit.
    TooLong(usize),
    /// The input contains a non-printable-ASCII character at the given byte offset.
    InvalidChar { offset: usize, ch: char },
    /// The input contains whitespace at the given byte offset.
    Whitespace { offset: usize, ch: char },
    /// The input does not contain a namespace separator (`:`).
    NoNamespace,
    /// The namespace (portion before `:`) is empty.
    EmptyNamespace,
    /// The local part (portion after `:`) is empty.
    EmptyLocalPart,
}

impl fmt::Display for RoomKeyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "room key must not be empty"),
            Self::TooLong(len) => {
                write!(f, "room key is {len} chars, maximum is {MAX_LEN}")
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
            Self::NoNamespace => {
                write!(
                    f,
                    "room key must contain a namespace separator ('{NAMESPACE_SEP}')"
                )
            }
            Self::EmptyNamespace => write!(f, "namespace portion must not be empty"),
            Self::EmptyLocalPart => write!(f, "local part (after ':') must not be empty"),
        }
    }
}

impl std::error::Error for RoomKeyError {}

impl RoomKey {
    /// Attempt to construct a `RoomKey` from a string, validating all invariants.
    pub fn try_new(s: impl Into<String>) -> Result<Self, RoomKeyError> {
        let s = s.into();

        if s.is_empty() {
            return Err(RoomKeyError::Empty);
        }
        if s.len() > MAX_LEN {
            return Err(RoomKeyError::TooLong(s.len()));
        }

        for (offset, ch) in s.char_indices() {
            if ch.is_whitespace() {
                return Err(RoomKeyError::Whitespace { offset, ch });
            }
            if !ch.is_ascii() || ch.is_ascii_control() {
                return Err(RoomKeyError::InvalidChar { offset, ch });
            }
        }

        let sep_pos = s.find(NAMESPACE_SEP).ok_or(RoomKeyError::NoNamespace)?;
        if sep_pos == 0 {
            return Err(RoomKeyError::EmptyNamespace);
        }
        if sep_pos == s.len() - 1 {
            return Err(RoomKeyError::EmptyLocalPart);
        }

        Ok(Self(s))
    }

    /// Returns the full key as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns the namespace portion (before the first `:`).
    pub fn namespace(&self) -> &str {
        // SAFETY: validated at construction — separator always present.
        self.0.split(NAMESPACE_SEP).next().unwrap()
    }

    /// Returns the local portion (after the first `:`).
    pub fn local_part(&self) -> &str {
        // SAFETY: validated at construction — separator always present.
        &self.0[self.0.find(NAMESPACE_SEP).unwrap() + 1..]
    }
}

impl TryFrom<String> for RoomKey {
    type Error = RoomKeyError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::try_new(s)
    }
}

impl TryFrom<&str> for RoomKey {
    type Error = RoomKeyError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::try_new(s)
    }
}

impl fmt::Display for RoomKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_key() {
        let key = RoomKey::try_new("catena:cantina/main").unwrap();
        assert_eq!(key.as_str(), "catena:cantina/main");
        assert_eq!(key.namespace(), "catena");
        assert_eq!(key.local_part(), "cantina/main");
    }

    #[test]
    fn valid_minimal() {
        let key = RoomKey::try_new("a:b").unwrap();
        assert_eq!(key.namespace(), "a");
        assert_eq!(key.local_part(), "b");
    }

    #[test]
    fn valid_with_multiple_colons() {
        let key = RoomKey::try_new("ns:room:sub").unwrap();
        assert_eq!(key.namespace(), "ns");
        assert_eq!(key.local_part(), "room:sub");
    }

    #[test]
    fn reject_empty() {
        assert_eq!(RoomKey::try_new(""), Err(RoomKeyError::Empty));
    }

    #[test]
    fn reject_too_long() {
        let long = format!("ns:{}", "a".repeat(MAX_LEN));
        let err = RoomKey::try_new(long.clone()).unwrap_err();
        assert!(matches!(err, RoomKeyError::TooLong(n) if n == long.len()));
    }

    #[test]
    fn reject_whitespace() {
        assert!(matches!(
            RoomKey::try_new("ns:room one"),
            Err(RoomKeyError::Whitespace { .. })
        ));
        assert!(matches!(
            RoomKey::try_new("ns:room\tone"),
            Err(RoomKeyError::Whitespace { .. })
        ));
        assert!(matches!(
            RoomKey::try_new("ns:room\none"),
            Err(RoomKeyError::Whitespace { .. })
        ));
    }

    #[test]
    fn reject_non_ascii() {
        assert!(matches!(
            RoomKey::try_new("ns:r\u{00f6}om"),
            Err(RoomKeyError::InvalidChar { .. })
        ));
    }

    #[test]
    fn reject_control_chars() {
        assert!(matches!(
            RoomKey::try_new("ns:room\x01"),
            Err(RoomKeyError::InvalidChar { .. })
        ));
    }

    #[test]
    fn reject_no_namespace_separator() {
        assert_eq!(
            RoomKey::try_new("justaplainkey"),
            Err(RoomKeyError::NoNamespace)
        );
    }

    #[test]
    fn reject_empty_namespace() {
        assert_eq!(
            RoomKey::try_new(":local"),
            Err(RoomKeyError::EmptyNamespace)
        );
    }

    #[test]
    fn reject_empty_local_part() {
        assert_eq!(RoomKey::try_new("ns:"), Err(RoomKeyError::EmptyLocalPart));
    }

    #[test]
    fn exactly_max_length_is_ok() {
        // 3 chars for "ns:" + 61 chars = 64 total
        let key_str = format!("ns:{}", "x".repeat(MAX_LEN - 3));
        assert_eq!(key_str.len(), MAX_LEN);
        assert!(RoomKey::try_new(key_str).is_ok());
    }

    #[test]
    fn one_over_max_length_rejected() {
        let key_str = format!("ns:{}", "x".repeat(MAX_LEN - 2));
        assert_eq!(key_str.len(), MAX_LEN + 1);
        assert!(matches!(
            RoomKey::try_new(key_str),
            Err(RoomKeyError::TooLong(_))
        ));
    }

    #[test]
    fn try_from_string() {
        let key: RoomKey = String::from("catena:void").try_into().unwrap();
        assert_eq!(key.as_str(), "catena:void");
    }

    #[test]
    fn try_from_str_ref() {
        let key: RoomKey = "catena:void".try_into().unwrap();
        assert_eq!(key.as_str(), "catena:void");
    }

    #[test]
    fn display() {
        let key = RoomKey::try_new("catena:cantina/main").unwrap();
        assert_eq!(format!("{key}"), "catena:cantina/main");
    }

    #[test]
    fn hash_and_eq() {
        use std::collections::HashSet;
        let a = RoomKey::try_new("catena:a").unwrap();
        let b = RoomKey::try_new("catena:a").unwrap();
        let c = RoomKey::try_new("catena:b").unwrap();
        let mut set = HashSet::new();
        set.insert(a.clone());
        assert!(set.contains(&b));
        assert!(!set.contains(&c));
    }
}
