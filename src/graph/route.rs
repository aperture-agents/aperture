//! routing node navigation through edges and conditional edges.
//!
//! after a node runs, the runtime must check the node's [`Router`] to decide the next step.
//! edges are just routers that ignore the state and unconditionally run.
//! conditional edge routers read state to determine the next step.

use crate::graph::node::{Next, NodeId};

/// pick next step from post-merge state
///
/// TODO: unclear - "implement for unconditional, fixed edges or domain logic (reading a `route: Option<NodeId` field the node just wrote)"
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

/// conditional via closure
/// `F` refers to the function to execute to determine Next.
/// `S` refers to the state to observe in order to make the correct Next decision.
/// PhantomData<fn(&S)> used because a conditional route is generic over the function to run to
/// determine Next AND the graph's [`State`](crate::graph::state::State) it oberserves to make the decision.
/// TODO: rename this if this is conditional_route - test this.
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
    // TODO: comment this and revisit if this is a conditional_edge
    // Should become new_conditional_edge
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
