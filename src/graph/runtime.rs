//! synchronous runtime loop
//!
//! owns runnable nodes and drives `Graph` wiring
//!
//! 1. start at `graph.entry`
//! 2. run the node -> delta -> `Merge`
//! 3. ask that node's `Router` for `Next`
//! 4. repeat until `Next::End`

use std::collections::HashMap;

use crate::graph::definition::Graph;
use crate::graph::id::{Next, NodeId};
use crate::graph::node::Node;
use crate::graph::state::{Merge, State, StateDelta};

/// type-erased node handle for runtime
/// `S` is the graph-wide state snapshot every node reads. `D` is the delta type stored in
/// `HashMap<NodeId, Box<dyn Runnable<S, D>>>`
/// does not mean the delta is the same in that it changes the state the same way, just means that
/// the value of `D` differs but that it is still type delta
pub trait Runnable<S, D> {
    fn run(&self, state: &S) -> D;
}

/// concrete types implement `Node` so we lift them into the runtime map here
impl<N> Runnable<N::State, N::Delta> for N
where
    N: Node,
{
    fn run(&self, state: &N::State) -> N::Delta {
        Node::run(self, state)
    }
}

/// why did a run stop early?
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunError {
    /// `graph` makes reference to a node id with no registered impl
    UnknownNode(NodeId),

    /// node ran but has no outgoing route in graph definition (false end)
    MissingRoute(NodeId),
}

/// node registry and executor
pub struct Runtime<S, D> {
    nodes: HashMap<NodeId, Box<dyn Runnable<S, D>>>,
}

impl<S, D> Default for Runtime<S, D> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S, D> Runtime<S, D> {
    #[must_use]
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
        }
    }

    /// register a runnable node at `id`
    pub fn node<N>(&mut self, id: NodeId, node: N) -> &mut Self
    where
        N: Node<State = S, Delta = D> + 'static,
    {
        self.nodes.insert(id, Box::new(node));
        self
    }

    /// execute `graph` from `state` until `Next::End` or an error occurs
    pub fn run(&self, graph: &Graph<S>, mut state: S) -> Result<S, RunError>
    where
        S: State + Merge<D>,
        D: StateDelta,
    {
        let mut current = graph.entry;

        loop {
            let runnable = self
                .nodes
                .get(&current)
                .ok_or(RunError::UnknownNode(current))?;

            let delta = runnable.run(&state);
            state.merge(delta);

            let router = graph
                .router(current)
                .ok_or(RunError::MissingRoute(current))?;

            match router.route(&state) {
                Next::Node(next) => current = next,
                Next::End => return Ok(state),
            }
        }
    }
}
