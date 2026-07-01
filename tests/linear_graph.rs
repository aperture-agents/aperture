//! e2e smoke integration test of linear graph

use aperture::graph::Graph;
use aperture::graph::node::{Node, NodeId};
use aperture::graph::state::{Merge, State, StateDelta};

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

    let mut graph = Graph::build();
    graph.add_node(a, Inc);
    graph.add_node(b, Inc);

    graph.add_edge(NodeId::START, a);
    graph.add_edge(a, b);
    graph.add_edge(b, NodeId::END);

    let out = graph.run(Counter::default()).unwrap();
    assert_eq!(out.n, 2);
}
