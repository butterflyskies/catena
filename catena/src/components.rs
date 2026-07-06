//! ECS components referenced by the engine invariants.

use hecs::Entity;
use serde::{Deserialize, Serialize};

use crate::exits::Exits;

/// Locates an entity inside a room. The `Entity` value points to the room
/// entity (which must have `RoomName`, `Description`, and `Exits` components
/// and a corresponding entry in the room registry — invariant 2.1).
#[derive(Clone, Copy, Debug)]
pub struct InRoom(pub Entity);

/// Human-readable room name, displayed to players.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RoomName(pub String);

/// Room description text, displayed on `look`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Description(pub String);

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
