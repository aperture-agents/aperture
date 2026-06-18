//! graph definition: entry point, fixed edges, conditionals
//!
//! a `Graph` is a structure which answers what is next after node x carries out its purpose

use std::collections::HashMap;

use crate::graph::id::NodeId;
use crate::graph::route::{Fixed, Router};

pub struct Graph<S> {
    /// first "real" node after `NodeID::START`
    pub entry: NodeId,

    /// per-node successor. if node is runnable, it should be in here
    routes: HashMap<NodeId, Box<dyn Router<S>>>,
}

impl<S> Graph<S> {
    /// begin: build graph whose first step is `entry`
    #[must_use]
    pub fn new(entry: NodeId) -> Self {
        Self {
            entry,
            routes: HashMap::new(),
        }
    }

    /// this is a fixed edge:
    /// `from` always continues to `to`
    /// linear chains ought to be a repeated `.edge(a, b).edge(b, c)...` set of calls
    #[must_use]
    pub fn edge(mut self, from: NodeId, to: NodeId) -> Self {
        self.routes.insert(from, Box::new(Fixed(to)));
        self
    }

    /// this is a conditional edge:
    /// `from` delegates to `router` after it runs
    #[must_use]
    pub fn route(mut self, from: NodeId, router: impl Router<S> + 'static) -> Self {
        self.routes.insert(from, Box::new(router));
        self
    }

    /// lookup the router for a node
    pub fn router(&self, from: NodeId) -> Option<&dyn Router<S>> {
        self.routes.get(&from).map(|b| b.as_ref())
    }
}
