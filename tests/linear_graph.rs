//! e2e smoke test of linear graph

use aperture::graph::node::Node;
use aperture::graph::state::{Merge, State, StateDelta};
use aperture::graph::{Graph, NodeId, Runtime};

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
fn linear_graph_runs_until_end() {
    let a = NodeId("a");
    let b = NodeId("b");

    let graph = Graph::new(a).edge(a, b).edge(b, NodeId::END);

    let mut runtime = Runtime::new();
    runtime.node(a, Inc).node(b, Inc);

    let out = runtime.run(&graph, Counter::default()).unwrap();
    assert_eq!(out.n, 2);
}
