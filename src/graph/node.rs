//! nodes: read state and emit deltas
//!
//! a node is one unit of work in a superstep (or a single transition, it depends on how the
//! runtime schedules the graph). it takes in the current [`State`] snapshot by reference and
//! returns a [`StateDelta`]. it must not mutate the snapshot as-is, all output flows through the
//! delta, which the runtime folds.
//!
//! run: &State ──► Delta ──runtime──► Merge ──► next State
//!
//! when several nodes run in the same superstep, each produces its own delta. the runtime collects
//! them and merges into one successor state. as stated, the order this occurs in is a graph or
//! runtime-specific policy.

use crate::graph::state::{Merge, State, StateDelta};

/// a graph node is a read-only use of state and partial update via delta
/// associated types tie each node to the state shape it expects and the delta shape it emits. the
/// `State: Merge<Self::Delta>` bound ensures the runtime can always apply the node's output.
pub trait Node {
    /// graph state this node reads
    type State: State + Merge<Self::Delta>;

    /// partial update this node produces
    type Delta: StateDelta;

    /// run the node against the current state snapshot. implementations should treat `state` as ro.
    /// side-effects (for our purposes, model calls, tool use, mcp connections, i/o) belong inside
    /// here; the state change still comes back as a delta for the runtime to merge.
    fn run(&self, state: &Self::State) -> Self::Delta;
}
