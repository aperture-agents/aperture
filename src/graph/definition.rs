//! graph definition: nodes, edges, and entry point.
//!
//! build with [`Graph::build`], register nodes and wiring, then [`Graph::run`].

use std::collections::HashMap;
use std::collections::hash_map::Entry;

use crate::graph::node::{Next, Node, NodeId};
use crate::graph::route::{Edge, Router};
use crate::graph::runtime::{RunError, Runnable};
use crate::graph::state::{Merge, State, StateDelta};

/// errors for build operations
///
/// duplicate nodes are not allowed - causes routing issues
#[derive(Debug)]
pub enum BuildError {
    DuplicateNode(NodeId),
}

/// core graph struct and executor - nodes plus wiring in one place
/// central element of the library.
/// users define one graph and register nodes and edges accordingly.
/// represents a built valid graph
pub struct Graph<S, D> {
    nodes: HashMap<NodeId, Box<dyn Runnable<S, D>>>,
    routes: HashMap<NodeId, Box<dyn Router<S>>>,
}

/// core graph builder struct
/// responsible for building the graph and validating it
/// once built and validated it will become a [`Graph`]
pub struct GraphBuilder<S, D> {
    nodes: HashMap<NodeId, Box<dyn Runnable<S, D>>>,
    routes: HashMap<NodeId, Box<dyn Router<S>>>,
}

impl<S, D> GraphBuilder<S, D> {
    /// validate an assembled graph
    /// must_use ensures that the graph is ran.
    #[must_use]
    pub fn build() -> Graph<S, D> {
        Graph {
            nodes: HashMap::new(),
            routes: HashMap::new(),
        }
    }
}

impl<S, D> Graph<S, D> {
    /// register a runnable node at `id`.
    /// duplicate NodeIds are not allowed.
    pub fn add_node<N>(mut self, id: NodeId, node: N) -> Result<Self, BuildError>
    where
        N: Node<State = S, Delta = D> + 'static,
    {
        match self.nodes.entry(id) {
            Entry::Vacant(entry) => {
                entry.insert(Box::new(node));
                Ok(self)
            },
            Entry::Occupied(_)=> {
                return Err(BuildError::DuplicateNode(id)u)
            },
        }
    }

    /// register an edge: `from` always continues to `to`.
    pub fn add_edge(mut self, from: NodeId, to: NodeId) -> Self {
        self.routes.insert(from, Box::new(Edge(to)));
        self
    }

    /// register a conditional edge: `from` delegates to `router` after it runs.
    pub fn add_conditional_edge(mut self, from: NodeId, router: impl Router<S> + 'static) -> Self {
        self.routes.insert(from, Box::new(router));
        self
    }

    /// lookup the router for a node.
    pub fn router(&self, from: NodeId) -> Option<&dyn Router<S>> {
        self.routes.get(&from).map(|b| b.as_ref())
    }

    /// execute from `state` until [`crate::graph::id::Next::End`] or an error.
    /// requires an edge from [`NodeId::START`] to the first real node.
    pub fn run(&self, mut state: S) -> Result<S, RunError>
    where
        S: State + Merge<D>,
        D: StateDelta,
    {
        // resolve the first real node via the mandatory START edge
        let mut current = match self
            .router(NodeId::START)
            .ok_or(RunError::MissingEntry)?
            .route(&state)
        {
            Next::Node(id) => id,
            Next::End => return Ok(state),
        };

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

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default, Clone, Debug, PartialEq, Eq)]
    struct Counter {
        n: u64,
    }

    impl State for Counter {}
    impl StateDelta for Counter {}

    impl Merge for Counter {
        fn merge(&mut self, delta: Self) {
            self.n += delta.n;
        }
    }

    struct Inc;

    impl Node for Inc {
        type State = Counter;
        type Delta = Counter;

        fn run(&self, _state: &Self::State) -> Self::Delta {
            Counter { n: 1 }
        }
    }

    #[test]
    fn missing_entry() {
        // Ensure a graph thats missing START edge fails
        //
        // START -X- a ──▶ b ──▶ END
        //
        let a = NodeId("node a");
        let b = NodeId("node b");

        let mut graph = Graph::build().add_node(a, Inc).unwrap()
        .add_node(b, Inc).unwrap()
        .add_edge(a, b).
        .add_edge(b, NodeId::END);

        let out = graph.run(Counter::default());

        assert_eq!(out.unwrap_err(), RunError::MissingEntry)
    }

    #[test]
    fn missing_end() {
        // Ensure a graph thats missing END edge fails
        // Currently this will manifest in a Runtime Error RunError::MissingRoute
        // Do we want this? Or do we want a more explicit pre-emptive check
        //
        // START ──▶ a ──▶ b ──▶ ???
        //
        let a = NodeId("a");
        let b = NodeId("b");

        let mut graph = Graph::build();

        graph.add_node(a, Inc);
        graph.add_node(b, Inc);

        graph.add_edge(NodeId::START, a);
        graph.add_edge(a, b);

        let out = graph.run(Counter::default());

        assert_eq!(out.unwrap_err(), RunError::MissingRoute(b));
    }

    #[test]
    fn missing_edge() {
        // Ensure a graph with a missing connection edge fails
        //
        // START ──▶ a ──▶ b ──▶ ???
        //
        // c ──▶ d ──▶ END   (unreachable island)
        //
        let a = NodeId("a");
        let b = NodeId("b");
        let c = NodeId("c");
        let d = NodeId("d");

        let mut graph = Graph::build();

        graph.add_node(a, Inc);
        graph.add_node(b, Inc);
        graph.add_node(c, Inc);
        graph.add_node(d, Inc);

        graph.add_edge(NodeId::START, a);
        graph.add_edge(a, b);
        graph.add_edge(c, d);
        graph.add_edge(d, NodeId::END);

        let out = graph.run(Counter::default());

        assert_eq!(out.unwrap_err(), RunError::MissingRoute(b));
    }

    #[test]
    fn empty_graph() {
        // Ensure an empty graph returns Error
        let graph = Graph::<Counter, Counter>::build();

        let out = graph.run(Counter::default());

        assert_eq!(out.unwrap_err(), RunError::MissingEntry)
    }

    #[test]
    fn duplicate_node() {
        // Tests an illegal node
        let a = NodeId("a");
        let illegal = NodeId("__end__");
        let b = NodeId("b");

        let mut graph = Graph::build();

        graph.add_node(a, Inc);
        graph.add_node(illegal, Inc);
        graph.add_node(b, Inc);

        graph.add_edge(NodeId::START, a);
        graph.add_edge(a, illegal);
        graph.add_edge(illegal, b);
        graph.add_edge(b, NodeId::END);

        let out = graph.run(Counter::default());

        assert_eq!(out.unwrap().n, 3);
}

#[test]
fn simple_increment() {
    // Ensure a simple linear graph works
    //
    // START ──▶ a ──▶ b ──▶ END
    //
    let a = NodeId("a");
    let b = NodeId("b");

    let mut graph = Graph::build();
    graph.add_node(a, Inc);
    graph.add_node(b, Inc);

    graph.add_edge(NodeId::START, a);
    graph.add_edge(a, b);
    graph.add_edge(b, NodeId::END);

    let out = graph.run(Counter::default());

    assert_eq!(out.unwrap().n, 2);
}

#[test]
fn spanning_graph() {
    // Ensuring a spanning graph works properly - expected to fail with current implementation
    //
    //           ┌──▶ a ──┐
    //           │        │
    // START ────┼──▶ b ──┼──▶ END
    //           │        │
    //           ├──▶ c ──┤
    //           │        │
    //           └──▶ d ──┘
    //
    let a = NodeId("a");
    let b = NodeId("b");
    let c = NodeId("c");
    let d = NodeId("d");

    let mut graph = Graph::build();
    graph.add_node(a, Inc);
    graph.add_node(b, Inc);
    graph.add_node(c, Inc);
    graph.add_node(d, Inc);

    graph.add_edge(NodeId::START, a);
    graph.add_edge(NodeId::START, b);
    graph.add_edge(NodeId::START, c);
    graph.add_edge(NodeId::START, d);

    graph.add_edge(a, NodeId::END);
    graph.add_edge(b, NodeId::END);
    graph.add_edge(c, NodeId::END);
    graph.add_edge(d, NodeId::END);

    let out = graph.run(Counter::default());

    assert_eq!(out.unwrap().n, 4);
}
}
