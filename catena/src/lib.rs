//! Catena engine — a MUD engine with formally verified invariants.
//!
//! This crate defines the core types, components, and data structures that the
//! engine guarantees. The invariants are documented in `INVARIANTS.md` and will
//! be verified via kani proof harnesses.

pub mod components;
pub mod direction;
pub mod exits;
pub mod room_key;
pub mod room_registry;
pub mod types;
