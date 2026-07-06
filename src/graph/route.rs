//! routing node navigation through edges and conditional edges.
//!
//! after a node runs, the runtime must check the node's [`Router`] to decide the next step.
//! edges are just routers that ignore the state and unconditionally run.
//! conditional edge routers read state to determine the next step.

use crate::graph::node::{Next, NodeId};

/// trait for edge types to implement
/// pick next step from post-merge state
///
/// an edge is defined as a path between nodes
/// as such routing frmo one node to another will give us a Next
/// Next is the next Node to run or in case of END - graph termination
///
pub trait Router<S> {
    fn route(&self, state: &S) -> Next;
}

/// edge always routes to same target, so we can use `NodeId::END` as `to` for a terminal node
#[derive(Clone, Copy, Debug)]
pub struct Edge(pub NodeId);

impl<S> Router<S> for Edge {
    fn route(&self, _state: &S) -> Next {
        Next::from_node(self.0)
    }
}

/// conditional route which selects Next from a closure result
/// `F` refers to the function to execute to determine Next.
/// `S` refers to the state to observe in order to make the correct Next decision.
/// PhantomData<fn(&S)> used because FnRouter is generic over state but does not contain a state.
///
pub struct FnRouter<S, F>
where
    F: Fn(&S) -> Next,
{
    f: F,
    _marker: std::marker::PhantomData<fn(&S)>,
}

impl<S, F> FnRouter<S, F>
where
    F: Fn(&S) -> Next,
{
    pub fn new(f: F) -> Self {
        Self {
            f,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<S, F> Router<S> for FnRouter<S, F>
where
    F: Fn(&S) -> Next,
{
    fn route(&self, state: &S) -> Next {
        (self.f)(state)
    }
}
