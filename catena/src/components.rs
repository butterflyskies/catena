//! ECS components referenced by the engine invariants.

use std::fmt;

use hecs::Entity;
use serde::{Deserialize, Serialize};

use crate::exits::Exits;

/// Locates an entity inside a room. The `Entity` value points to the room
/// entity (which must have `RoomName`, `Description`, and `Exits` components
/// and a corresponding entry in the room registry — invariant 2.1).
#[derive(Clone, Copy, Debug)]
pub struct InRoom(pub Entity);

/// Human-readable room name, displayed to players.
///
/// Construction is fallible — names must be non-empty and within
/// [`RoomName::MAX_LEN`] characters.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(into = "String", try_from = "String")]
pub struct RoomName(String);

/// Reasons a string cannot be a valid [`RoomName`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RoomNameError {
    Empty,
    TooLong(usize),
}

impl fmt::Display for RoomNameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "room name must not be empty"),
            Self::TooLong(len) => {
                write!(f, "room name is {len} chars, maximum is {}", RoomName::MAX_LEN)
            }
        }
    }
}

impl std::error::Error for RoomNameError {}

impl RoomName {
    /// Maximum length of a room name in characters.
    pub const MAX_LEN: usize = 200;

    pub fn new(name: impl Into<String>) -> Result<Self, RoomNameError> {
        let name = name.into();
        if name.is_empty() {
            return Err(RoomNameError::Empty);
        }
        if name.len() > Self::MAX_LEN {
            return Err(RoomNameError::TooLong(name.len()));
        }
        Ok(Self(name))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<RoomName> for String {
    fn from(n: RoomName) -> Self {
        n.0
    }
}

impl TryFrom<String> for RoomName {
    type Error = RoomNameError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

impl fmt::Display for RoomName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Room description text, displayed on `look`.
///
/// Construction is fallible — descriptions must be within
/// [`Description::MAX_LEN`] characters. Empty descriptions are allowed (a room
/// can be indescribable).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(into = "String", try_from = "String")]
pub struct Description(String);

/// Reasons a string cannot be a valid [`Description`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DescriptionError {
    TooLong(usize),
}

impl fmt::Display for DescriptionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLong(len) => {
                write!(
                    f,
                    "description is {len} chars, maximum is {}",
                    Description::MAX_LEN
                )
            }
        }
    }
}

impl std::error::Error for DescriptionError {}

impl Description {
    /// Maximum length of a room description in characters.
    pub const MAX_LEN: usize = 8_000;

    pub fn new(text: impl Into<String>) -> Result<Self, DescriptionError> {
        let text = text.into();
        if text.len() > Self::MAX_LEN {
            return Err(DescriptionError::TooLong(text.len()));
        }
        Ok(Self(text))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<Description> for String {
    fn from(d: Description) -> Self {
        d.0
    }
}

impl TryFrom<String> for Description {
    type Error = DescriptionError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

impl fmt::Display for Description {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Exits attached to a room entity. See [`Exits`] for the opaque API.
pub type RoomExits = Exits;

/// Marker component: this entity is a unique singleton. No other live entity
/// may share its content identity key (invariant 2.6).
#[derive(Clone, Copy, Debug, Default)]
pub struct Unique;

/// Binds a player entity to its network session (invariant 3.1).
#[derive(Debug)]
pub struct PlayerSession {
    /// Unique session ID (invariant 3.4).
    pub session_id: u64,
}

/// Optional custom state blob attached to an entity (invariant 10.1).
/// The engine persists the bytes with round-trip fidelity; schema migration
/// is the mudlib's responsibility (invariant 10.2).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CustomState(pub Vec<u8>);

/// Opt-in marker: this entity receives periodic tick callbacks (invariant 6.2).
/// Entities without this component incur zero scheduler overhead.
#[derive(Clone, Copy, Debug, Default)]
pub struct Ticker;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn room_name_valid() {
        let name = RoomName::new("The Cantina").unwrap();
        assert_eq!(name.as_str(), "The Cantina");
    }

    #[test]
    fn room_name_reject_empty() {
        assert_eq!(RoomName::new(""), Err(RoomNameError::Empty));
    }

    #[test]
    fn room_name_reject_too_long() {
        let long = "x".repeat(RoomName::MAX_LEN + 1);
        assert!(matches!(RoomName::new(&long), Err(RoomNameError::TooLong(_))));
    }

    #[test]
    fn room_name_exactly_max_ok() {
        let name = "x".repeat(RoomName::MAX_LEN);
        assert!(RoomName::new(&name).is_ok());
    }

    #[test]
    fn room_name_serde_round_trip() {
        let name = RoomName::new("The Void").unwrap();
        let json = serde_json::to_string(&name).unwrap();
        let back: RoomName = serde_json::from_str(&json).unwrap();
        assert_eq!(name, back);
    }

    #[test]
    fn description_valid() {
        let desc = Description::new("A dimly lit room.").unwrap();
        assert_eq!(desc.as_str(), "A dimly lit room.");
    }

    #[test]
    fn description_allows_empty() {
        assert!(Description::new("").is_ok());
    }

    #[test]
    fn description_reject_too_long() {
        let long = "x".repeat(Description::MAX_LEN + 1);
        assert!(matches!(Description::new(&long), Err(DescriptionError::TooLong(_))));
    }

    #[test]
    fn description_serde_round_trip() {
        let desc = Description::new("You are in a maze of twisty passages.").unwrap();
        let json = serde_json::to_string(&desc).unwrap();
        let back: Description = serde_json::from_str(&json).unwrap();
        assert_eq!(desc, back);
    }
}
