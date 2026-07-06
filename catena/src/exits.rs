//! Opaque exit map for rooms.
//!
//! Invariant 1.9: The `Exits` component is an opaque newtype wrapping the
//! direction-to-room-key mapping. Mutation goes through the `Exits` API.
//! The backing container is private. Bidirectional consistency (invariant 1.2)
//! is the mudlib's responsibility — the mudlib calls `set()` on both rooms
//! to create a two-way exit.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::direction::Direction;
use crate::room_key::RoomKey;

/// An opaque exit map for a single room.
///
/// The inner `HashMap` is not publicly accessible. Each `set()` call adds one
/// directional exit. To create a bidirectional passage, the mudlib calls
/// `set()` on both rooms. The engine does not generate reciprocal patches.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Exits {
    map: HashMap<Direction, RoomKey>,
}

impl Exits {
    /// Create an empty exit map.
    pub fn new() -> Self {
        Self::default()
    }

    /// Look up where a direction leads.
    pub fn get(&self, direction: &Direction) -> Option<&RoomKey> {
        self.map.get(direction)
    }

    /// Returns `true` if this exit map contains the given direction.
    pub fn contains(&self, direction: &Direction) -> bool {
        self.map.contains_key(direction)
    }

    /// How many exits this room has.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Whether this room has no exits.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Iterate over all (direction, room_key) pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&Direction, &RoomKey)> {
        self.map.iter()
    }

    /// Set an exit in the given direction. Returns the previous target if one
    /// existed (displacement).
    ///
    /// This sets a single directional exit. For a bidirectional passage, the
    /// mudlib calls `set()` on both the source and destination rooms.
    pub fn set(&mut self, direction: Direction, target: RoomKey) -> Option<RoomKey> {
        self.map.insert(direction, target)
    }

    /// Remove an exit. Returns the target it pointed to, or `None` if no exit
    /// existed in this direction.
    pub fn remove(&mut self, direction: &Direction) -> Option<RoomKey> {
        self.map.remove(direction)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(s: &str) -> RoomKey {
        RoomKey::try_new(s).unwrap()
    }

    fn north() -> Direction {
        Direction::new("north").unwrap()
    }

    fn south() -> Direction {
        Direction::new("south").unwrap()
    }

    fn east() -> Direction {
        Direction::new("east").unwrap()
    }

    #[test]
    fn set_and_get() {
        let mut exits = Exits::new();
        let target = key("catena:room-b");

        let displaced = exits.set(north(), target.clone());
        assert!(displaced.is_none());
        assert_eq!(exits.get(&north()), Some(&target));
    }

    #[test]
    fn set_displacement() {
        let mut exits = Exits::new();
        let old_target = key("catena:room-b");
        let new_target = key("catena:room-c");

        exits.set(north(), old_target.clone());
        let displaced = exits.set(north(), new_target.clone());

        assert_eq!(displaced, Some(old_target));
        assert_eq!(exits.get(&north()), Some(&new_target));
    }

    #[test]
    fn bidirectional_via_two_sets() {
        let mut exits_a = Exits::new();
        let mut exits_b = Exits::new();
        let key_a = key("catena:a");
        let key_b = key("catena:b");

        // Mudlib creates the bidirectional link explicitly
        exits_a.set(north(), key_b.clone());
        exits_b.set(south(), key_a.clone());

        assert_eq!(exits_a.get(&north()), Some(&key_b));
        assert_eq!(exits_b.get(&south()), Some(&key_a));
    }

    #[test]
    fn one_way_exit() {
        let mut exits_a = Exits::new();
        let exits_b = Exits::new();
        let key_b = key("catena:trap");

        // Only set one direction — inherently one-way
        exits_a.set(north(), key_b.clone());

        assert_eq!(exits_a.get(&north()), Some(&key_b));
        assert!(exits_b.get(&south()).is_none());
    }

    #[test]
    fn remove_existing() {
        let mut exits = Exits::new();
        let target = key("catena:b");

        exits.set(north(), target.clone());
        let removed = exits.remove(&north());

        assert_eq!(removed, Some(target));
        assert!(exits.get(&north()).is_none());
    }

    #[test]
    fn remove_nonexistent() {
        let mut exits = Exits::new();
        assert!(exits.remove(&north()).is_none());
    }

    #[test]
    fn len_and_is_empty() {
        let mut exits = Exits::new();
        assert!(exits.is_empty());
        assert_eq!(exits.len(), 0);

        exits.set(north(), key("catena:a"));
        assert!(!exits.is_empty());
        assert_eq!(exits.len(), 1);
    }

    #[test]
    fn iter_yields_all_exits() {
        let mut exits = Exits::new();
        exits.set(north(), key("catena:a"));
        exits.set(east(), key("catena:b"));

        let collected: Vec<_> = exits.iter().collect();
        assert_eq!(collected.len(), 2);
    }

    #[test]
    fn contains() {
        let mut exits = Exits::new();
        assert!(!exits.contains(&north()));

        exits.set(north(), key("catena:a"));
        assert!(exits.contains(&north()));
    }
}
