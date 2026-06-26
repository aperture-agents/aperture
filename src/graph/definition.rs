//! graph definition: nodes, edges, and entry point.
//!
//! build with [`Graph::build`], register nodes and wiring, then [`Graph::run`].

use std::collections::HashMap;

use crate::graph::id::NodeId;
use crate::graph::node::Node;
use crate::graph::route::{Fixed, Router};
use crate::graph::runtime::{RunError, Runnable};
use crate::graph::state::{Merge, State, StateDelta};

/// graph builder and executor - nodes plus wiring in one place
pub struct Graph<S, D> {
    entry: Option<NodeId>,
    nodes: HashMap<NodeId, Box<dyn Runnable<S, D>>>,
    routes: HashMap<NodeId, Box<dyn Router<S>>>,
}

impl<S, D> Graph<S, D> {
    /// empty graph; add nodes and edges, then run.
    #[must_use]
    pub fn build() -> Self {
        Self {
            entry: None,
            nodes: HashMap::new(),
            routes: HashMap::new(),
        }
    }

    /// set the first node to run after [`NodeId::START`].
    ///
    /// if unset, the first [`add_node`](Self::add_node) call becomes the entry.
    pub fn set_entry(&mut self, entry: NodeId) -> &mut Self {
        self.entry = Some(entry);
        self
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

    /// fixed edge: `from` always continues to `to`.
    pub fn add_edge(&mut self, from: NodeId, to: NodeId) -> &mut Self {
        self.routes.insert(from, Box::new(Fixed(to)));
        self
    }

    /// conditional edge: `from` delegates to `router` after it runs.
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
        use crate::graph::id::Next;

        let mut current = self.entry.ok_or(RunError::MissingEntry)?;

        loop {
            let runnable = self
                .nodes
                .get(&current)
                .ok_or(RunError::UnknownNode(current))?;

            let delta = runnable.run(&state);
            state.merge(delta);

            let router = self
                .router(current)
                .ok_or(RunError::MissingRoute(current))?;

            match router.route(&state) {
                Next::Node(next) => current = next,
                Next::End => return Ok(state),
            }
        }
    }
}
