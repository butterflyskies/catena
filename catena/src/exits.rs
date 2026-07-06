//! Opaque exit map enforcing bidirectional consistency.
//!
//! Invariant 1.9: The `Exits` component is an opaque newtype wrapping the
//! direction-to-room-key mapping. Mutation goes through the `Exits` API, which
//! enforces invariant 1.2 (bidirectional consistency) by returning the operations
//! the caller must apply to the destination room. The backing container is private.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::direction::Direction;
use crate::room_key::RoomKey;

/// An opaque exit map for a single room.
///
/// The inner `HashMap` is not publicly accessible. All mutations go through
/// methods that return [`ExitPatch`] values describing the reciprocal
/// operations the caller must apply to maintain bidirectional consistency
/// (invariant 1.2).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Exits {
    map: HashMap<Direction, RoomKey>,
}

/// A reciprocal operation that must be applied to a destination room's exits
/// to maintain bidirectional consistency (invariant 1.2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExitPatch {
    /// The destination room should add an exit in the given direction pointing
    /// to the given room key.
    Add {
        direction: Direction,
        target: RoomKey,
    },
    /// The destination room should remove the exit in the given direction.
    Remove { direction: Direction },
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

    /// Add a two-way exit. Returns an [`ExitPatch::Add`] that the caller must
    /// apply to the destination room to complete the bidirectional link.
    ///
    /// If an exit already existed in this direction, the old target is displaced.
    /// The second element of the returned tuple carries an [`ExitPatch::Remove`]
    /// that the caller must apply to the **old** target to clean up its stale
    /// reciprocal. When no displacement occurs this is `None`.
    pub fn set(
        &mut self,
        direction: Direction,
        target: RoomKey,
        self_key: &RoomKey,
    ) -> (ExitPatch, Option<(RoomKey, ExitPatch)>) {
        let old = self.map.insert(direction.clone(), target);
        let add_patch = ExitPatch::Add {
            direction: direction.inverse(),
            target: self_key.clone(),
        };
        let displacement = old.map(|old_target| {
            let remove_patch = ExitPatch::Remove {
                direction: direction.inverse(),
            };
            (old_target, remove_patch)
        });
        (add_patch, displacement)
    }

    /// Add a one-way exit (no reciprocal patch). Per invariant 1.2, a one-way
    /// exit is one where the destination simply has no corresponding exit back.
    pub fn set_one_way(&mut self, direction: Direction, target: RoomKey) {
        self.map.insert(direction, target);
    }

    /// Remove an exit. Returns an [`ExitPatch::Remove`] that the caller must
    /// apply to the destination room to clean up the reciprocal link.
    ///
    /// Returns `None` if no exit existed in this direction.
    pub fn remove(&mut self, direction: &Direction) -> Option<(RoomKey, ExitPatch)> {
        let target = self.map.remove(direction)?;
        let patch = ExitPatch::Remove {
            direction: direction.inverse(),
        };
        Some((target, patch))
    }

    /// Remove an exit without producing a reciprocal patch. Used internally
    /// when applying an [`ExitPatch::Remove`] from another room.
    pub fn remove_one_way(&mut self, direction: &Direction) -> Option<RoomKey> {
        self.map.remove(direction)
    }

    /// Apply an [`ExitPatch`] received from another room's mutation.
    pub fn apply_patch(&mut self, patch: ExitPatch) {
        match patch {
            ExitPatch::Add { direction, target } => {
                self.map.insert(direction, target);
            }
            ExitPatch::Remove { direction } => {
                self.map.remove(&direction);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(s: &str) -> RoomKey {
        RoomKey::try_new(s).unwrap()
    }

    fn north() -> Direction {
        Direction::standard("north").unwrap()
    }

    fn south() -> Direction {
        Direction::standard("south").unwrap()
    }

    #[test]
    fn set_produces_reciprocal_patch() {
        let mut exits = Exits::new();
        let self_key = key("catena:room-a");
        let target_key = key("catena:room-b");

        let (patch, displacement) = exits.set(north(), target_key.clone(), &self_key);

        // Our exit map now has north -> room-b
        assert_eq!(exits.get(&north()), Some(&target_key));

        // The patch tells room-b to add south -> room-a
        assert_eq!(
            patch,
            ExitPatch::Add {
                direction: south(),
                target: self_key,
            }
        );

        // No displacement — slot was empty
        assert!(displacement.is_none());
    }

    #[test]
    fn apply_patch_creates_reciprocal_exit() {
        let mut exits_a = Exits::new();
        let mut exits_b = Exits::new();
        let key_a = key("catena:a");
        let key_b = key("catena:b");

        let (patch, _) = exits_a.set(north(), key_b.clone(), &key_a);
        exits_b.apply_patch(patch);

        // A has north -> B
        assert_eq!(exits_a.get(&north()), Some(&key_b));
        // B has south -> A
        assert_eq!(exits_b.get(&south()), Some(&key_a));
    }

    #[test]
    fn set_displacement_returns_old_target() {
        let mut exits = Exits::new();
        let self_key = key("catena:a");
        let old_target = key("catena:b");
        let new_target = key("catena:c");

        // First set — no displacement
        let (_, displacement) = exits.set(north(), old_target.clone(), &self_key);
        assert!(displacement.is_none());

        // Overwrite north with a new target — old target displaced
        let (add_patch, displacement) = exits.set(north(), new_target.clone(), &self_key);

        // The add patch points the new target back to us
        assert_eq!(
            add_patch,
            ExitPatch::Add {
                direction: south(),
                target: self_key.clone(),
            }
        );

        // Displacement tells caller to clean up room-b's stale reciprocal
        let (displaced_key, remove_patch) = displacement.unwrap();
        assert_eq!(displaced_key, old_target);
        assert_eq!(remove_patch, ExitPatch::Remove { direction: south() });

        // Our exit map points north -> room-c now
        assert_eq!(exits.get(&north()), Some(&new_target));
    }

    #[test]
    fn remove_produces_reciprocal_patch() {
        let mut exits = Exits::new();
        let self_key = key("catena:a");
        let target_key = key("catena:b");

        exits.set(north(), target_key.clone(), &self_key);
        let (removed_target, patch) = exits.remove(&north()).unwrap();

        assert_eq!(removed_target, target_key);
        assert_eq!(exits.get(&north()), None);
        assert_eq!(patch, ExitPatch::Remove { direction: south() });
    }

    #[test]
    fn remove_nonexistent_returns_none() {
        let mut exits = Exits::new();
        assert!(exits.remove(&north()).is_none());
    }

    #[test]
    fn one_way_exit_no_patch() {
        let mut exits = Exits::new();
        exits.set_one_way(north(), key("catena:trap"));
        assert_eq!(exits.get(&north()), Some(&key("catena:trap")));
    }

    #[test]
    fn len_and_is_empty() {
        let mut exits = Exits::new();
        assert!(exits.is_empty());
        assert_eq!(exits.len(), 0);

        exits.set_one_way(north(), key("catena:a"));
        assert!(!exits.is_empty());
        assert_eq!(exits.len(), 1);
    }

    #[test]
    fn iter_yields_all_exits() {
        let mut exits = Exits::new();
        exits.set_one_way(north(), key("catena:a"));
        exits.set_one_way(Direction::standard("east").unwrap(), key("catena:b"));

        let collected: Vec<_> = exits.iter().collect();
        assert_eq!(collected.len(), 2);
    }
}
