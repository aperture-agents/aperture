//! node type-erasure and run errors
//!
//! [`super::definition::Graph`] owns the synchronous execution loop; this module holds the
//! plumbing it uses to store heterogeneous node implementations in one map

use crate::graph::node::Node;

/// type-erased node handle for the runtime registry
///
/// `S` is the graph-wide state snapshot every node reads. `D` is the delta type stored in
/// `HashMap<NodeId, Box<dyn Runnable<S, D>>>` - a simplification, not a model rule.
/// [`Node`] still allows per-node `Delta` types; the registry forces one shared `D` so the loop
/// can call a single `state.merge(delta)` without node-specific dispatch
///
/// same `D` does not mean nodes change state the same way. they return different *values* of `D`
/// (e.g. enum variants, optional fields); [`Merge`](crate::graph::state::Merge) decides how each
/// applies
pub trait Runnable<S, D> {
    fn run(&self, state: &S) -> D;
}

impl<N> Runnable<N::State, N::Delta> for N
where
    N: Node,
{
    fn run(&self, state: &N::State) -> N::Delta {
        Node::run(self, state)
    }
}

/// why a run stopped early
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunError {
    /// no entry point set and no nodes registered
    MissingEntry,

    /// graph references a node id with no registered implementation
    UnknownNode(crate::graph::id::NodeId),

    /// node ran but has no outgoing route in the graph definition
    MissingRoute(crate::graph::id::NodeId),
}
