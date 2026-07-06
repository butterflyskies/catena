//! Enums and value types referenced by engine invariants.

use serde::{Deserialize, Serialize};

/// Handler disposition returned by command handlers (invariant 5.5).
///
/// Controls how the engine's two-pass dispatch chain proceeds after each
/// handler executes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Disposition {
    /// Stop evaluation — this handler produced the definitive response.
    Final,
    /// Apply this handler's effects as a layer and continue to lower-priority
    /// handlers.
    Continue,
    /// This handler did not handle the command. Keep looking.
    Fail,
}

/// Container reload state machine (invariant 4.1).
///
/// The only legal transition sequence is:
/// `Active -> Staged -> Draining -> Swapped`.
///
/// `Active -> Swapped` and `Draining -> Active` are illegal.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReloadState {
    /// The container is live and serving normally. Default state.
    Active,
    /// A replacement container has been prepared but not yet activated.
    Staged,
    /// The old container is emptying its occupants before swap.
    /// Transition to `Swapped` is blocked while occupancy > 0 (invariant 4.2).
    Draining,
    /// The swap is complete. The old container entity will be despawned
    /// (invariant 4.4).
    Swapped,
}

/// Policy decision returned by mudlib drain callbacks (invariant 4.7).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SwapDecision {
    /// Proceed with the drain/swap step.
    Proceed,
    /// Defer this step — try again next tick.
    Defer,
    /// Force the step regardless of occupancy or other conditions.
    Force,
}

/// The effect declaration returned by a command handler (invariant 5.4).
///
/// Handlers return effects rather than mutating world state directly.
/// The engine applies all effects atomically after the handler chain completes.
///
/// Currently carries only output text. A `mutations` field (world-state changes
/// such as move-entity, set-attribute, destroy-item) will be added once the
/// engine's apply-effects pass is implemented.
#[derive(Clone, Debug, Default)]
pub struct Effects {
    /// Output text to send to the acting player.
    pub output: Vec<String>,
    /// Output text to broadcast to other entities in the room.
    pub broadcast: Vec<String>,
}
