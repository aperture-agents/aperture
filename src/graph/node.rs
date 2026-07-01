//! nodes: read state and emit deltas
//!
//! graphs are keyed by `NodeId`
//!
//! non-work nodes are what langgraph calls sentinels? so
//! `NodeId::START` and `NodeID::END` are sentinels and therefore not runnable nodes
//!
//! a node is one unit of work in a superstep (or a single transition, it depends on how the
//! runtime schedules the graph). it takes in the current [`State`] snapshot by reference and
//! returns a [`StateDelta`]. it must not mutate the [`State`] as-is, all output flows through the
//! [`StateDelta`], the runtime will then manage the merging of [`StateDelta`] into [`State`].
//!
//! run: &State ──► Delta ──runtime──► Merge ──► next State
//!
//! when several nodes run in the same superstep, each produces its own delta. the runtime collects
//! them and merges into one successor state. as stated, the order this occurs in is a graph or
//! runtime-specific policy.

use crate::graph::state::{Merge, State, StateDelta};

/// a graph node contains a shared reference of state and outputs a partial update via delta.
/// associated types tie each node to the state shape it expects and the delta shape it emits.
/// the `State: Merge<Self::Delta>` bound ensures the runtime can always apply the node's output.
/// This ensures a Node defines some bare minimum Merge implemention on its fields.
pub trait Node {
    /// graph state this node reads
    /// an associated type because a Graph will only ever use one unified State
    /// as such only one implementation of Node for State is expected
    /// as such associated types are preferred to generic types
    type State: State + Merge<Self::Delta>;

    /// partial update this node produces
    /// again an associated type because a given Node will only ever return one type of delta
    /// as such only one implementation of Node with a given delta is expected
    /// as such associated types are preferred to generic types
    type Delta: StateDelta;

    /// run the node against the current state. State will be a shared reference/
    /// side-effects (for our purposes, model calls, tool use, mcp connections, i/o) belong inside
    /// here; the state change still comes back as a delta for the runtime to merge.
    fn run(&self, state: &Self::State) -> Self::Delta;
}

/// unique node id for a slot in a graph def
/// helps the runtime discern and fetch nodes.
/// unique market nodes like START and END are responsible for marking runtime start and ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NodeId(pub &'static str);

impl NodeId {
    /// graph entry; runtime jumps to the graph's entry node, not execution of `__start__`
    pub const START: Self = Self("__start__");

    /// graph exit, ends run
    pub const END: Self = Self("__end__");
}

/// where to go after current node finishes and delta merged?
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Next {
    /// continue at other node
    Node(NodeId),

    /// stop (`NodeId::END`)
    End,
}

impl Next {
    /// normalize `NodeId::END` into `Next::End`
    #[must_use]
    pub fn from_node(id: NodeId) -> Self {
        if id == NodeId::END {
            Self::End
        } else {
            Self::Node(id)
        }
    }
}
