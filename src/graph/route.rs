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
/// as such routing from one node to another will give us a Next
/// Next is the next Node to run or in case of END - graph termination
///
pub trait Edge<S> {
    fn route(&self, state: &S) -> Next;
    fn possible_next(&self) -> Vec<Next>;
}

/// trait for expected route targets an edge can return
/// edges must return RouteTargets
///
/// this allows us to get variants() aka next_possible
///
pub trait RouteTargets: Copy {
    fn variants() -> Vec<Next>;
}

/// an unconditonal edge always routes to same target, so we can use `NodeId::END` as `to` for a terminal node
#[derive(Clone, Copy, Debug)]
pub struct UnconditionalEdge(pub NodeId);

impl<S> Edge<S> for UnconditionalEdge {
    fn route(&self, _state: &S) -> Next {
        Next::from_node(self.0)
    }

    fn possible_next(&self) -> Vec<Next> {
        vec![Next::from_node(self.0)] // unconditional edge - only one possible next
    }
}

/// conditional route which selects Next from a closure result
/// `F` refers to the function to execute to determine Next.
/// `S` refers to the state to observe in order to make the correct Next decision.
/// `R` refers to the possible route targets this edge can expect to route to.
/// PhantomData<fn(&S) -> R> used because ConditionalEdge is generic over state and routetargets but does not contain a state.
///
pub struct ConditionalEdge<S, F, R>
where
    F: Fn(&S) -> R,
    R: RouteTargets + Into<Next>,
{
    f: F,
    _marker: std::marker::PhantomData<fn(&S) -> R>,
}

impl<S, F, R> ConditionalEdge<S, F, R>
where
    F: Fn(&S) -> R,
    R: RouteTargets + Into<Next>,
{
    pub fn new(f: F) -> Self {
        Self {
            f,
            _marker: std::marker::PhantomData::<fn(&S) -> R>,
        }
    }
}

impl<S, F, R> Edge<S> for ConditionalEdge<S, F, R>
where
    F: Fn(&S) -> R,
    R: RouteTargets + Into<Next>,
{
    fn route(&self, state: &S) -> Next {
        (self.f)(state).into()
    }

    fn possible_next(&self) -> Vec<Next> {
        R::variants()
    }
}
