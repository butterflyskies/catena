//! ECS components referenced by the engine invariants.

use hecs::Entity;
use nutype::nutype;
use serde::{Deserialize, Serialize};

use crate::exits::Exits;

/// Locates an entity inside a room. The `Entity` value points to the room
/// entity (which must have `RoomName`, `Description`, and `Exits` components
/// and a corresponding entry in the room registry — invariant 2.1).
#[derive(Clone, Copy, Debug)]
pub struct InRoom(pub Entity);

/// Human-readable room name, displayed to players.
///
/// Construction is fallible — names must be non-empty and within 200 characters.
#[nutype(
    validate(not_empty, len_char_max = 200),
    derive(Clone, Debug, PartialEq, Eq, Display, AsRef, Into, TryFrom, Serialize, Deserialize)
)]
pub struct RoomName(String);

/// Room description text, displayed on `look`.
///
/// Construction is fallible — descriptions must be within 8000 characters.
/// Empty descriptions are allowed (a room can be indescribable).
#[nutype(
    validate(len_char_max = 8000),
    derive(Clone, Debug, PartialEq, Eq, Display, AsRef, Into, TryFrom, Serialize, Deserialize)
)]
pub struct Description(String);

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
        let name = RoomName::try_new("The Cantina").unwrap();
        assert_eq!(name.as_ref(), "The Cantina");
    }

    #[test]
    fn room_name_reject_empty() {
        assert!(RoomName::try_new("").is_err());
    }

    #[test]
    fn room_name_reject_too_long() {
        let long = "x".repeat(201);
        assert!(RoomName::try_new(&long).is_err());
    }

    #[test]
    fn room_name_exactly_max_ok() {
        let name = "x".repeat(200);
        assert!(RoomName::try_new(&name).is_ok());
    }

    #[test]
    fn room_name_serde_round_trip() {
        let name = RoomName::try_new("The Void").unwrap();
        let json = serde_json::to_string(&name).unwrap();
        let back: RoomName = serde_json::from_str(&json).unwrap();
        assert_eq!(name, back);
    }

    #[test]
    fn description_valid() {
        let desc = Description::try_new("A dimly lit room.").unwrap();
        assert_eq!(desc.as_ref(), "A dimly lit room.");
    }

    #[test]
    fn description_allows_empty() {
        assert!(Description::try_new("").is_ok());
    }

    #[test]
    fn description_reject_too_long() {
        let long = "x".repeat(8001);
        assert!(Description::try_new(&long).is_err());
    }

    #[test]
    fn description_serde_round_trip() {
        let desc = Description::try_new("You are in a maze of twisty passages.").unwrap();
        let json = serde_json::to_string(&desc).unwrap();
        let back: Description = serde_json::from_str(&json).unwrap();
        assert_eq!(desc, back);
    }
}
