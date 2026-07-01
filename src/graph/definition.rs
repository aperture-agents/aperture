//! graph definition: nodes, edges, and entry point.
//!
//! build with [`Graph::build`], register nodes and wiring, then [`Graph::run`].

use std::collections::HashMap;

use crate::graph::node::{Node, NodeId, Next};
use crate::graph::route::{Edge, Router};
use crate::graph::runtime::{RunError, Runnable};
use crate::graph::state::{Merge, State, StateDelta};

/// core graph builder and executor - nodes plus wiring in one place
/// central element of the library.
/// users define one graph and register nodes and edges accordingly.
pub struct Graph<S, D> {
    entry: Option<NodeId>,
    nodes: HashMap<NodeId, Box<dyn Runnable<S, D>>>,
    routes: HashMap<NodeId, Box<dyn Router<S>>>,
}

impl<S, D> Graph<S, D> {
    /// empty graph; add nodes and edges, then run.
    /// must_use ensures that the graph is ran.
    #[must_use]
    pub fn build() -> Self {
        Self {
            entry: None,
            nodes: HashMap::new(),
            routes: HashMap::new(),
        }
    }

    /// register a runnable node at `id`.
    pub fn add_node<N>(&mut self, id: NodeId, node: N) -> &mut Self
    where
        N: Node<State = S, Delta = D> + 'static,
    {
        if self.entry.is_none() {
            self.entry = Some(id);
        }
        self.nodes.insert(id, Box::new(node));
        self
    }

    /// register an edge: `from` always continues to `to`.
    pub fn add_edge(&mut self, from: NodeId, to: NodeId) -> &mut Self {
        self.routes.insert(from, Box::new(Edge(to)));
        self
    }

    /// register a conditional edge: `from` delegates to `router` after it runs.
    pub fn add_conditional_edge(
        &mut self,
        from: NodeId,
        router: impl Router<S> + 'static,
    ) -> &mut Self {
        self.routes.insert(from, Box::new(router));
        self
    }

    /// lookup the router for a node.
    pub fn router(&self, from: NodeId) -> Option<&dyn Router<S>> {
        self.routes.get(&from).map(|b| b.as_ref())
    }

    /// execute from `state` until [`crate::graph::id::Next::End`] or an error.
    pub fn run(&self, mut state: S) -> Result<S, RunError>
    where
        S: State + Merge<D>,
        D: StateDelta,
    {
        // fail if START not registered with a node
        let mut current = self.entry.ok_or(RunError::MissingEntry)?;

        loop {
            // get the current node
            let runnable = self
                .nodes
                .get(&current)
                .ok_or(RunError::UnknownNode(current))?;

            // run the node and get the delta
            let delta = runnable.run(&state);

            // merge the delta into the state
            state.merge(delta);

            // get the router for the current node
            let router = self
                .router(current)
                .ok_or(RunError::MissingRoute(current))?;

            // route to the next node or return the final state
            match router.route(&state) {
                Next::Node(next) => current = next,
                Next::End => return Ok(state),
            }
        }
    }
}
