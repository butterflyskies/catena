# Catena Engine — Invariants and Properties

Formal properties the catena engine must uphold. Each invariant is numbered,
precisely stated, and intended to be provable via kani harnesses. The engine
guarantees these; the mudlib may rely on them.

Terminology:
- **RoomKey**: a validated, namespaced string identifier (e.g. `"catena:cantina/main"`).
- **Room registry**: the authoritative map from RoomKey → current room entity.
- **Container**: any entity that can hold other entities. A room is a container with exits. A backpack is a container without exits.
- **WorldApi**: the engine-provided interface through which mudlib code interacts with the ECS.

---

## 1. Room Graph Integrity

**1.1** Every exit stored in any room's `Exits` component references a `RoomKey`
that exists in the room registry.

**1.2** If room A has an exit in direction D pointing to room B, then room B has
a corresponding exit in the inverse direction pointing to A — unless the exit is
explicitly marked one-way. The mechanism for marking an exit as one-way is the
absence of a reciprocal exit in the destination room's `Exits` component.

**1.3** Self-referencing exits (a room with an exit pointing to itself) are
permitted. Circular corridors and treadmill rooms are valid game design.

**1.4** The designated spawn point is always a valid `RoomKey` that resolves to a
live room entity in the registry. The spawn point is never empty, never dangling.

**1.5** Every `RoomKey` in the registry is unique. No two registry entries share
the same key.

**1.6** After any mutating operation on the room graph (add room, remove room,
add exit, remove exit), invariants 1.1–1.5 continue to hold.

**1.7** `RoomKey` construction validates syntax: non-empty, contains a namespace
separator, valid characters only. Invalid keys are rejected at creation and never
stored in the registry.

---

## 2. Entity Location

**2.1** Every entity that has an `InRoom` component points to an entity that is a
live, valid room (i.e. has `RoomName` + `Description` components and a
corresponding `RoomKey` in the registry).

**2.2** Movement is transactional. The move function atomically updates `InRoom`
— it never returns with the entity removed from the source but not yet placed in
the destination. A located entity is in exactly one room at all times — never
zero, never two simultaneously.

**2.3** Containment traversal uses the visitor pattern with a seen-set for cycle
detection. Containment cycles are permitted (a bag of holding inside a room
inside the bag is valid game design). Traversal never causes infinite recursion —
visited entities are skipped. The engine provides the walker; the mudlib provides
the visitor implementation.

**2.4** The count of entities whose `InRoom` points to a given room entity is
always equal to the room's reported occupancy. No bookkeeping drift. If the
engine introduces a cached occupancy count, it must always equal the count of
entities whose `InRoom` points to the room. Invariant 4.2 depends on this count.

**2.5** After movement, the player's available command set reflects the providers
in their new location. Commands from the previous location are no longer
available.

**2.6** If an entity has the `Unique` marker component, no other live entity in
the world shares its content identity key. `spawn_from_prefab(key)` fails if
`Unique` is specified and an instance already exists.

---

## 3. Session Lifecycle

**3.1** Accepting a connection always produces exactly one player entity with both
`InRoom` and `PlayerSession` components. No connection exists without a
corresponding entity.

**3.2** Disconnection (clean quit or network drop) always despawns the player
entity and releases all associated resources. No orphan player entities persist
after disconnect.

**3.3** No two player entities share the same write channel (`WriteTx`). Each
connection has a unique, non-aliased output path.

**3.4** Session IDs are unique across the lifetime of the server process. No two
sessions, concurrent or sequential, share an ID.

**3.5** A player entity's `InRoom` is always set to a valid room before the
player receives any game output. There is no window where a player entity exists
without a location.

**3.6** The set of player entities with `InRoom` pointing to room R is always
consistent with the set of active sessions the room considers present.

---

## 4. Hot-Swap / Drain-Then-Swap (Containers)

A room is a container with exits. The invariants in this section apply to any
entity with containment semantics — rooms, backpacks, pocket dimensions, or any
future container type. The engine treats all containers uniformly; the mudlib
decides what each container means.

**4.1** The container reload state machine permits only the following transition
sequence: `Active → Staged → Draining → Swapped`. No other transitions are
valid. In particular, `Active → Swapped` and `Draining → Active` are illegal.

**4.2** A container in `Draining` state never transitions to `Swapped` while its
occupancy is greater than zero.

**4.3** During the `Draining` phase, new arrivals (via movement or placement)
resolve to the old (currently active) container instance, not the staged
replacement. Entities in the same logical container always see the same instance.

**4.4** After the `Swapped` transition completes, the old container entity is
despawned. No zombie container entities persist in the ECS after their
replacement is active.

**4.5** Because exits reference `RoomKey`s (not entity IDs), exit consistency
across hot-swap is maintained by construction. The room registry atomically
updates the key → entity mapping on swap. No exit ever resolves to a despawned
entity.

**4.6** If no reload is pending, a container remains in `Active` state
indefinitely. The state machine does not spontaneously advance.

**4.7** The engine emits lifecycle events during drain-then-swap:
`on_drain_start(old, new)`, `on_occupant_leave(old, entity)`,
`on_drain_complete(old, new)`, `on_swap(new)`. The mudlib registers callbacks
that return policy decisions (proceed, defer, force). The engine handles the
transaction; the mudlib decides what goes where (item migration, NPC respawning,
timer transfer, combat blocking).

---

## 5. Command Dispatch

**5.1** `dispatch()` never panics for any input. Empty strings, unknown commands,
and malformed input all produce a well-formed error response, never a crash.

**5.2** Every invocation of a registered command's `execute()` method receives a
player entity that exists in the ECS and has valid `InRoom` and `PlayerSession`
components.

**5.3** `register()` and `unregister()` are idempotent. Registering an
already-registered command replaces it. Unregistering a non-existent command is a
no-op.

**5.4** Command resolution is two-pass: the engine first collects all matching
handlers from all providers, sorted by numeric priority. Then handlers execute in
order, each receiving an immutable world view and returning an `Effects`
declaration (output text, proposed mutations, conditional hints). The engine
applies all effects atomically after the chain completes. No handler directly
mutates world state.

**5.5** Each handler returns a disposition: `Final` (stop evaluation, this is the
answer), `Continue` (apply as a layer, keep checking lower-priority handlers), or
`Fail` (I didn't handle this, keep looking). The engine respects these
dispositions during the execution pass.

**5.6** The engine does not impose a fixed provider hierarchy. The mudlib owns
priority assignment. The engine guarantees highest numeric priority wins with
deterministic tiebreak (registration order within same priority).

**5.7** If no handler matches or all return `Fail`, the engine returns a
"command not found" response. It does not silently drop the input.

**5.8** Command registration and unregistration are atomic with movement, wield,
and unwield transactions. When an entity enters or leaves a room, or an item is
equipped or unequipped, the affected command providers are updated in the same
tick. No window exists where commands are available but the entity is not present,
or vice versa.

---

## 6. Scheduler / Timers

**6.1** Every scheduled timer eventually either fires or is explicitly cancelled.
No timer remains permanently queued without resolution.

**6.2** Entities without an opt-in `Ticker` component incur zero scheduler
overhead. The default is silent; ticking is pay-for-what-you-use.

**6.3** Timer and tick callbacks execute within a contained context. A panic or
error in a callback does not propagate to the engine's main loop, does not affect
other timers, and does not corrupt world state.

**6.4** Timer firing order respects scheduled time. Timers scheduled for the same
tick fire in insertion order (FIFO). No starvation, no reordering.

**6.5** The default tick rate is 1 Hz (one tick per second), configurable by the
mudlib. The minimum tick interval (floor) is 50ms (20 Hz). The engine refuses to
start with a tick rate below the floor.

**6.6** Under load, if a tick takes longer than its budget, the engine skips
missed ticks rather than catching up. The tick counter advances monotonically —
no tick fires twice. Skipped ticks are logged. Timers due during skipped ticks
are batched into the next executed tick.

---

## 7. Security / Privilege

**7.1** Permission enforcement uses casbin with ABAC (attribute-based access
control) semantics, surfaced to the mudlib through the WorldApi. Auth providers
and policy configuration are engine-level operational concerns. The mudlib
registers policy rules; the engine evaluates them via casbin. No script, command,
or mudlib code path can grant itself privileges without authorization from an
entity that already holds the required capability.

**7.2** Mudlib code interacts with the world exclusively through the `WorldApi`
boundary. Direct ECS access (`hecs::World`) is not exposed to mudlib or script
code.

**7.3** Script execution (Rhai) is sandboxed by default. Filesystem access and
network access are available but require explicit capability grants
(`FileCapability`, `NetCapability`) on the acting entity. No capability = no I/O.
The engine mediates all capability-gated operations. Unsafe Rust code and direct
engine internal mutation remain unconditionally prohibited.

**7.4** The `WorldApi` enforces capability checks before executing privileged
operations (room creation/deletion, entity teleportation, server shutdown,
session inspection, save/restore).

**7.5** Capabilities are namespace-scoped, using the same namespace convention as
RoomKeys and content identity. `builder:catena:cantina/*` grants build
permissions within the cantina zone. `admin:*` is unrestricted. Granularity is
configurable by the mudlib's policy rules.

---

## 8. Error Containment

**8.1** A failing script (panic, runtime error, infinite loop timeout) is caught
by the interpreter boundary. The engine logs the error, the offending action is
skipped, and all other systems continue normally.

**8.2** A network error on one connection (read failure, write failure, protocol
violation) does not affect any other connection. Connections are fully isolated.

**8.3** A malformed content file (invalid Rhai syntax, missing required
components, schema violation) is rejected at load/reload time with a diagnostic
error. It never reaches runtime execution. A failed reload leaves the previous
revision active.

**8.4** An error in a timer callback does not prevent subsequent timers from
firing. The scheduler logs the error and continues draining the queue.

**8.5** Malformed telnet sequences are handled by libmudtelnet's parser and
cannot corrupt parser state, leak data between connections, or crash the server.

**8.6** The engine maintains a void room — a hardcoded, engine-owned room that
always exists, cannot be modified, hot-swapped, or destroyed by any mudlib
operation. It is the universal fallback: players land in the void when their room
is despawned, their `InRoom` is invalid, or their save references a room that no
longer exists. The engine guarantees arrival at the void; the mudlib provides
recovery commands (respawn, go home, admin teleport) for leaving it.

**8.7** A player is never stranded in an unrecoverable state without an escape
path. If their room fails, they are moved to the void. If their session fails,
they can reconnect. If their save fails, the previous save is intact. There is
always a path back to a known-good state.

**8.8** Error blast radius is scoped: a broken room affects all occupants of that
room. A broken player affects one connection. A broken script affects the entity
running it. Failures never propagate beyond their scope.

**8.9** Save failures are ops events, not silent fallbacks. A failed save logs an
error, notifies the admin channel, and preserves the previous good save. The
player's data is never lost due to a save failure.

---

## 9. Serialization / Persistence

**9.1** Serialization and deserialization of world state is round-trip safe.
`load(save(world))` produces a world satisfying all invariants in sections 1
and 2. (Placeholder — full spec when save/load is implemented.)

---

## 10. Entity State Persistence

**10.1** Each entity may have an optional `CustomState` blob (serializable data).
The engine handles save/load of this blob with byte fidelity —
`load(save(state))` produces identical bytes.

**10.2** Schema compatibility and migration of `CustomState` is the mudlib
script's responsibility. The engine provides an `on_load(old_state)` hook; the
script handles upgrades between versions.

**10.3** A failed migration (script error in `on_load`) is contained — the entity
loads with its old state intact, the error is logged, and the world continues.

---

## 11. Authentication

**11.1** The engine owns authentication: provider configuration (local, Discord,
GitHub, OIDC), token validation, session binding, and principal identity.
Supported auth providers are an operational concern configured at the engine
level, not by the mudlib.

**11.2** The engine delivers an authenticated principal to the mudlib. The mudlib
owns the mapping from principal → player(s) → character(s). Auth is
infrastructure; identity is game logic.

**11.3** Principal ≠ player. One principal may own multiple players across
multiple catena instances. This indirection enables: federation between MUD
instances, multi-character play, bridge/relay principals, and test harness
simulation of multiple players from a single auth identity.

**11.4** For self-hosted auth, the engine uses oxide-auth as an OAuth2 provider.
For external providers, the engine uses the oauth2 crate. Both are engine
internals not exposed to mudlib code.

---

## Verification Strategy

Each invariant maps to one or more kani proof harnesses. The harnesses model
engine operations (add room, move entity, connect/disconnect, hot-swap, dispatch
command, fire timer) and assert that the invariant holds after every operation
sequence.

Invariants 1.x, 2.x, and 3.x are co-dependent foundations — session lifecycle
creates the entities that location invariants constrain. Prove these together
first.

Invariants 4.x (hot-swap) are the most complex state machine and the highest
bug-potential area. Prove these second.

Invariants 5.x, 6.x are dispatch/scheduler — prove third.

Invariants 7.x, 8.x, and 9.x are boundary and persistence properties — prove
last, as they depend on the Rhai integration and save/load layers which may not
be in place for the initial build.

### Verification scope notes

Invariants 3.1, 3.2, 3.5 describe async lifecycle properties. Kani targets the
synchronous state-transition functions (e.g., `connect()` always calls
`spawn()`). The async boundaries require stateright verification or hand-audit.

Invariant 8.1's infinite-loop timeout is a runtime liveness property outside
kani's scope. Test via integration tests with timeout enforcement.
