//! graph definition: nodes, edges, and entry point.
//!
//! build with [`Graph::build`], register nodes and wiring, then [`Graph::run`].

use std::collections::HashMap;
use std::fmt;

use crate::graph::node::{Next, Node, NodeId};
use crate::graph::route::{Edge, Router};
use crate::graph::runtime::{RunError, Runnable};
use crate::graph::state::{Merge, State, StateDelta};

/// errors for build operations
///
/// duplicate nodes are not allowed - causes routing issues
#[derive(Debug, PartialEq)]
pub enum BuildError {
    DuplicateNode(NodeId),
    MissingRoute(NodeId),
    MissingExit,
    MissingEntry,
    InvalidEntry,
    InvalidExit,
}

/// core graph struct and executor - nodes plus wiring in one place
/// central element of the library.
/// users define one graph and register nodes and edges accordingly.
/// represents a built and validated graph
pub struct Graph<S, D> {
    nodes: HashMap<NodeId, Box<dyn Runnable<S, D>>>,
    routes: HashMap<NodeId, Box<dyn Router<S>>>,
}

/// core graph builder struct
/// responsible for building the graph and validating it
/// build() will validate and construct a runnable [`Graph`]
pub struct GraphBuilder<S, D> {
    nodes: HashMap<NodeId, Box<dyn Runnable<S, D>>>,
    routes: HashMap<NodeId, Box<dyn Router<S>>>,
}

impl<S, D> GraphBuilder<S, D> {
    /// register a runnable node at `id`.
    pub fn add_node<N>(mut self, id: NodeId, node: N) -> Self
    where
        N: Node<State = S, Delta = D> + 'static,
    {
        self.nodes.insert(id, Box::new(node));
        self
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

    /// validate an assembled graph
    /// must_use ensures that the graph is ran.
    #[must_use]
    pub fn build(self) -> Result<Graph<S, D>, BuildError> {
        // START has an outgoing edge (START -> a)
        if !self.routes.contains_key(&NodeId::START) {
            return Err(BuildError::MissingEntry);
        }
        // END does not have an outgoing edge (End -> a)
        if self.routes.contains_key(&NodeId::END) {
            return Err(BuildError::InvalidExit);
        }
        // TODO: START does not have an incoming edge (a -> START)
        // TODO: END has an incoming edge (a -> END)
        // TODO: Other Validation...

        Ok(Graph {
            nodes: self.nodes,
            routes: self.routes,
        })
    }
}

impl<S, D> fmt::Debug for Graph<S, D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Graph")
            .field("nodes", &self.nodes.len())
            .field("routes", &self.routes.len())
            .finish()
    }
}

impl<S, D> Graph<S, D> {
    /// create the builder instance for constructing the graph
    pub fn builder() -> GraphBuilder<S, D> {
        GraphBuilder {
            nodes: HashMap::new(),
            routes: HashMap::new(),
        }
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

        let graph = Graph::builder()
            .add_node(a, Inc)
            .add_node(b, Inc)
            .add_edge(a, b)
            .add_edge(b, NodeId::END)
            .build();

        assert_eq!(graph.unwrap_err(), BuildError::MissingEntry)
    }

    #[test]
    fn entry_invalid_use() {
        // Ensure a user cant route to START - only from START
        // TODO: Validation check not implemented here yet
        //
        // START -> a -> START
        //
        let a = NodeId("node a");

        let graph = Graph::builder()
            .add_node(a, Inc)
            .add_edge(NodeId::START, a)
            .add_edge(a, NodeId::START)
            .build();

        assert_eq!(graph.unwrap_err(), BuildError::InvalidEntry)
    }

    #[test]
    fn end_invalid_use() {
        // Ensure a user cant route from END - only to END
        // TODO: Validation check not implemented here yet
        //
        // START -> a -> END -> a
        //
        let a = NodeId("node a");

        let graph = Graph::builder()
            .add_node(a, Inc)
            .add_edge(NodeId::START, a)
            .add_edge(a, NodeId::END)
            .add_edge(NodeId::END, a)
            .build();

        assert_eq!(graph.unwrap_err(), BuildError::InvalidExit)
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

        let graph = Graph::builder()
            .add_node(a, Inc)
            .add_node(b, Inc)
            .add_edge(NodeId::START, a)
            .add_edge(a, b)
            .build();

        assert_eq!(graph.unwrap_err(), BuildError::MissingExit);
    }

    #[test]
    fn missing_edge() {
        // Ensure a graph with a missing connection edge fails
        // TODO: Currently fails due to no validation for walking the graph
        //
        // START ──▶ a ──▶ b ──▶ ???
        //
        // c ──▶ d ──▶ END   (unreachable island)
        //
        let a = NodeId("a");
        let b = NodeId("b");
        let c = NodeId("c");
        let d = NodeId("d");

        let graph = Graph::builder()
            .add_node(a, Inc)
            .add_node(b, Inc)
            .add_node(c, Inc)
            .add_node(d, Inc)
            .add_edge(NodeId::START, a)
            .add_edge(a, b)
            .add_edge(c, d)
            .add_edge(d, NodeId::END)
            .build();

        assert_eq!(graph.unwrap_err(), BuildError::MissingRoute(b));
    }

    #[test]
    fn empty_graph() {
        // Ensure an empty graph returns Error
        // For now this is Missing Start Node but should this be a seperate error?
        //
        let graph = Graph::<Counter, Counter>::builder().build();

        assert_eq!(graph.unwrap_err(), BuildError::MissingEntry)
    }

    #[test]
    fn duplicate_node() {
        // Tests an illegal node
        //
        // START ──▶ a ──▶ illegal (__END__) -> b -> END
        //
        // TODO: currently fails due to using END as a Edge Key - This SHOULD Fail but probably during Node creation
        let a = NodeId("a");
        let illegal = NodeId("__end__");
        let b = NodeId("b");

        let graph = Graph::builder()
            .add_node(a, Inc)
            .add_node(illegal, Inc)
            .add_node(b, Inc)
            .add_edge(NodeId::START, a)
            .add_edge(a, illegal)
            .add_edge(illegal, b)
            .add_edge(b, NodeId::END)
            .build();

        assert_eq!(graph.unwrap_err(), BuildError::InvalidExit);
    }

    #[test]
    fn simple_increment() {
        // Ensure a simple linear graph works
        //
        // START ──▶ a ──▶ b ──▶ END
        //
        let a = NodeId("a");
        let b = NodeId("b");

        let graph = Graph::builder()
            .add_node(a, Inc)
            .add_node(b, Inc)
            .add_edge(NodeId::START, a)
            .add_edge(a, b)
            .add_edge(b, NodeId::END)
            .build()
            .unwrap();

        let out = graph.run(Counter::default());

        assert_eq!(out.unwrap().n, 2);
    }

    #[test]
    fn spanning_graph() {
        // Ensuring a spanning graph works properly
        // TODO: expected to fail with current implementation
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

        let graph = Graph::builder()
            .add_node(a, Inc)
            .add_node(b, Inc)
            .add_node(c, Inc)
            .add_node(d, Inc)
            .add_edge(NodeId::START, a)
            .add_edge(NodeId::START, b)
            .add_edge(NodeId::START, c)
            .add_edge(NodeId::START, d)
            .add_edge(a, NodeId::END)
            .add_edge(b, NodeId::END)
            .add_edge(c, NodeId::END)
            .add_edge(d, NodeId::END)
            .build()
            .unwrap();

        let out = graph.run(Counter::default());

        assert_eq!(out.unwrap().n, 4);
    }

    #[test]
    fn basic_conditional() {
        // Ensure a basic conditional edge works
        todo!()
    }

    #[test]
    fn advanced_conditional() {
        // Ensure a conditional works for more advanced state comparisons
    }

    #[test]
    fn conditional_exit() {
        // Ensure conditionals can exit
    }

    #[test]
    fn looping_conditional() {
        // Ensure we can invoke some recursion with conditionals
    }

    #[test]
    fn invalid_conditional_entry() {
        // Ensure conditionals cant loop back to START
    }

}
