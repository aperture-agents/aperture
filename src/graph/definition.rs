//! graph definition: nodes, edges, and entry point.
//!
//! build with [`Graph::build`], register nodes and wiring, then [`Graph::run`].

use std::collections::{HashMap, HashSet, VecDeque};
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

/// private core graph builder struct
/// responsible for building the graph and validating it
/// build() will validate and construct a runnable [`Graph`]
pub struct GraphBuilder<S, D> {
    nodes: HashMap<NodeId, Box<dyn Runnable<S, D>>>,
    routes: HashMap<NodeId, Box<dyn Router<S>>>,
    error: Option<BuildError>,
}

impl<S, D> GraphBuilder<S, D> {
    /// register a runnable node at `id`.
    pub fn add_node<N>(mut self, id: impl Into<NodeId>, node: N) -> Self
    where
        N: Node<State = S, Delta = D> + 'static,
    {
        let id = id.into();
        if self.nodes.insert(id, Box::new(node)).is_some() {
            self.error.get_or_insert(BuildError::DuplicateNode(id));
        }
        self
    }

    /// register an edge: `from` always continues to `to`.
    pub fn add_edge(mut self, from: impl Into<NodeId>, to: impl Into<NodeId>) -> Self {
        let f_id = from.into();
        let t_id = to.into();

        // START cannot be a to
        if t_id == NodeId::START {
            self.error.get_or_insert(BuildError::InvalidEntry);
        }

        // END cannot be a from
        if f_id == NodeId::END {
            self.error.get_or_insert(BuildError::InvalidExit);
        }
        self.routes.insert(f_id, Box::new(Edge(t_id)));
        self
    }

    /// register a conditional edge: `from` delegates to `router` after it runs.
    /// NOTE: No current clean way to validate the router possibilites and if they're valid
    pub fn add_conditional_edge(
        mut self,
        from: impl Into<NodeId>,
        router: impl Router<S> + 'static,
    ) -> Self {
        let f_id = from.into();
        let invalid_start = router
            .possible_next()
            .iter()
            .any(|p| *p == Next::from_node("__start__"));

        // START cannot be a to
        if invalid_start {
            self.error.get_or_insert(BuildError::InvalidEntry);
        }

        // END cannot be a from
        if f_id == NodeId::END {
            self.error.get_or_insert(BuildError::InvalidExit);
        }

        self.routes.insert(f_id, Box::new(router));
        self
    }

    /// validate an assembled graph
    /// Result is #[must_use] this ensures the graph is handled and ran
    pub fn build(self) -> Result<Graph<S, D>, BuildError> {
        // Check for any errors found during build steps
        if let Some(err) = self.error {
            return Err(err);
        }

        // Ensure START has an outgoing edge (START -> a)
        if !self.routes.contains_key(&NodeId::START) {
            return Err(BuildError::MissingEntry);
        }

        // Ensure END does not have an outgoing edge (End -> a)
        if self.routes.contains_key(&NodeId::END) {
            return Err(BuildError::InvalidExit);
        }

        // every registered node must have an outgoing route
        for id in self.nodes.keys() {
            if !self.routes.contains_key(id) {
                return Err(BuildError::MissingRoute(*id));
            }
        }

        // Walk the graph to ensure every node can be reached via some edge
        // Also ensure that END can be reached via some edge
        // walk every router's declared possible_next()
        let mut visited = HashSet::new();
        let mut queue = VecDeque::from([NodeId::START]);
        let mut end_reachable = false;

        while let Some(current) = queue.pop_front() {
            if !visited.insert(current) {
                continue;
            }
            let Some(router) = self.routes.get(&current) else {
                continue; // no outgoing route from here — already caught above for real nodes
            };

            for next in router.possible_next() {
                match next {
                    Next::Node(target) => {
                        if target != NodeId::END && !self.nodes.contains_key(&target) {
                            // a conditional edge declares a target that was never add_node'd
                            return Err(BuildError::MissingRoute(target));
                        }
                        queue.push_back(target);
                    }
                    Next::End => end_reachable = true,
                }
            }
        }

        if !end_reachable {
            return Err(BuildError::MissingExit);
        }

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
            error: None,
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
        n: i64,
    }

    impl Counter {
        pub fn new(n: i64) -> Self {
            Self { n }
        }
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

    struct Dec;

    impl Node for Dec {
        type State = Counter;
        type Delta = Counter;

        fn run(&self, _state: &Self::State) -> Self::Delta {
            Counter { n: -1 }
        }
    }

    struct EndIfZero;

    impl Router<Counter> for EndIfZero {
        fn route(&self, state: &Counter) -> Next {
            if state.n == 0 {
                return Next::End;
            } else {
                return Next::from_node("a");
            }
        }

        fn possible_next(&self) -> Vec<Next> {
            vec![Next::from_node("a"), Next::End]
        }
    }

    struct ToStart;

    impl Router<Counter> for ToStart {
        fn route(&self, _state: &Counter) -> Next {
            Next::from_node("__start__")
        }

        fn possible_next(&self) -> Vec<Next> {
            vec![Next::from_node("__start__")]
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
            .add_edge(b, a)
            .build();

        assert_eq!(graph.unwrap_err(), BuildError::MissingExit);
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
        let graph = Graph::builder()
            .add_node("a", Inc)
            .add_node("b", Inc)
            .add_edge(NodeId::START, "a")
            .add_edge("a", "b")
            .add_edge("b", NodeId::END)
            .build()
            .unwrap();

        let out = graph.run(Counter::default());

        assert_eq!(out.unwrap().n, 2);
    }

    #[test]
    #[ignore = "expected to fail with current minimal implementation"]
    fn spanning_graph() {
        // Ensuring a spanning graph works properly
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
        //
        //           ┌──▶ a ───────▶ END
        //           │
        // START ────┤
        //           │
        //           └─────────────▶ END
        //
        let a = NodeId("a");

        let graph = Graph::builder()
            .add_node(a, Dec)
            .add_conditional_edge(NodeId::START, EndIfZero)
            .add_edge(a, NodeId::END)
            .build()
            .unwrap();

        let out = graph.run(Counter::new(2));

        assert_eq!(out.unwrap().n, 1);
    }

    #[test]
    fn looping_conditional() {
        // Ensure we can invoke some recursion with conditionals
        //        n != 0
        //   ┌────────────────┐
        //   │                ▼
        //   │      START ──▶ a  ─┐
        //   │                    │
        //   └────────────────────┘
        //  (a routes back through DecUntilZero)
        //
        //              n == 0
        //   START ──────────────▶ END
        //
        let a = NodeId("a");

        let graph = Graph::builder()
            .add_node(a, Dec)
            .add_edge(NodeId::START, a)
            .add_conditional_edge("a", EndIfZero)
            .build()
            .unwrap();

        let out = graph.run(Counter::new(10));

        assert_eq!(out.unwrap().n, 0);
    }

    #[test]
    fn invalid_conditional_exit() {
        // Ensure conditionals cant route from END
        //
        // END ──────────────▶ ???
        //
        let graph = Graph::<Counter, Counter>::builder()
            .add_conditional_edge(NodeId::END, EndIfZero)
            .build()
            .unwrap_err();

        assert_eq!(graph, BuildError::InvalidExit);
    }

    #[test]
    fn invalid_conditional_entry() {
        // Ensure conditionals cant route to START
        //
        // START ──────▶ a ──X──▶ START
        //
        // This is not allowed in our current design.
        //
        let graph = Graph::builder()
            .add_node("a", Inc)
            .add_edge(NodeId::START, "a")
            .add_conditional_edge("a", ToStart)
            .build()
            .unwrap_err();

        assert_eq!(graph, BuildError::InvalidEntry);
    }
}
