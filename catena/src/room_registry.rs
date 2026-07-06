//! Opaque room registry mapping `RoomKey` to ECS entity.
//!
//! Invariant 1.10: The room registry is an opaque newtype. The backing store is
//! private — callers use the public API (get, insert, remove, contains, len,
//! iter). This allows swapping the backing container without changing call sites.

use std::collections::HashMap;

use hecs::Entity;

use crate::room_key::RoomKey;

/// The authoritative map from [`RoomKey`] to the ECS entity representing that room.
///
/// The inner container is private. All access goes through the public API.
#[derive(Debug, Default)]
pub struct RoomRegistry {
    map: HashMap<RoomKey, Entity>,
}

impl RoomRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Look up the entity for a room key.
    pub fn get(&self, key: &RoomKey) -> Option<Entity> {
        self.map.get(key).copied()
    }

    /// Returns `true` if the registry contains the given key.
    pub fn contains(&self, key: &RoomKey) -> bool {
        self.map.contains_key(key)
    }

    /// Insert a room key → entity mapping. Returns the previous entity if the
    /// key was already registered (invariant 1.5 says keys are unique, so this
    /// is a replacement, e.g. during hot-swap per invariant 4.5).
    pub fn insert(&mut self, key: RoomKey, entity: Entity) -> Option<Entity> {
        self.map.insert(key, entity)
    }

    /// Remove a room key from the registry. Returns the entity that was
    /// registered, or `None` if the key was not present.
    pub fn remove(&mut self, key: &RoomKey) -> Option<Entity> {
        self.map.remove(key)
    }

    /// How many rooms are registered.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Iterate over all (key, entity) pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&RoomKey, &Entity)> {
        self.map.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(s: &str) -> RoomKey {
        RoomKey::try_new(s).unwrap()
    }

    /// Helper: create a fresh hecs::World and spawn an entity.
    fn spawn_entity(world: &mut hecs::World) -> Entity {
        world.spawn(())
    }

    #[test]
    fn insert_and_get() {
        let mut world = hecs::World::new();
        let mut reg = RoomRegistry::new();
        let k = key("catena:tavern");
        let e = spawn_entity(&mut world);

        assert!(reg.insert(k.clone(), e).is_none());
        assert_eq!(reg.get(&k), Some(e));
    }

    #[test]
    fn contains() {
        let mut world = hecs::World::new();
        let mut reg = RoomRegistry::new();
        let k = key("catena:plaza");
        let e = spawn_entity(&mut world);

        assert!(!reg.contains(&k));
        reg.insert(k.clone(), e);
        assert!(reg.contains(&k));
    }

    #[test]
    fn remove() {
        let mut world = hecs::World::new();
        let mut reg = RoomRegistry::new();
        let k = key("catena:dungeon");
        let e = spawn_entity(&mut world);

        reg.insert(k.clone(), e);
        let removed = reg.remove(&k);
        assert_eq!(removed, Some(e));
        assert!(!reg.contains(&k));
        assert!(reg.remove(&k).is_none());
    }

    #[test]
    fn len_and_is_empty() {
        let mut world = hecs::World::new();
        let mut reg = RoomRegistry::new();
        assert!(reg.is_empty());
        assert_eq!(reg.len(), 0);

        reg.insert(key("catena:a"), spawn_entity(&mut world));
        reg.insert(key("catena:b"), spawn_entity(&mut world));
        assert_eq!(reg.len(), 2);
        assert!(!reg.is_empty());

        reg.remove(&key("catena:a"));
        assert_eq!(reg.len(), 1);
    }
}
