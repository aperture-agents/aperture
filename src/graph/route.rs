//! routing fixed edges and conditionals
//!
//! after a node runs, the runtime must check the node's `Router` to decide the next step
//! fixed edges are just routers that ignore the state while conditional routers read state

use crate::graph::id::{Next, NodeId};

/// pick next step from post-merge state
///
/// implement for unconditional, fixed edges or domain logic (reading a `route: Option<NodeId` field
/// the node just wrote)
pub trait Router<S> {
    fn route(&self, state: &S) -> Next;
}

/// fixed edge always routes to same target, so we can use `NodeId::END` as `to` for a terminal node
#[derive(Clone, Copy, Debug)]
pub struct Fixed(pub NodeId);

impl<S> Router<S> for Fixed {
    fn route(&self, _state: &S) -> Next {
        Next::from_node(self.0)
    }
}

/// conditional via closure
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
