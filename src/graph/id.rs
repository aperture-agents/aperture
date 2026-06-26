//! node identity and control-flow
//!
//! graphs are keyed by `NodeId`
//!
//! i think non-work nodes are what langgraph calls sentinels? so
//! `NodeId::START` and `NodeID::END` are sentinels and therefore not runnable nodes

/// name for a node slot in a graph def
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NodeId(pub &'static str);

impl NodeId {
    /// graph entry; runtime jumps to the graph's entry node, not execution of `__start__`
    pub const START: Self = Self("__start__");

    /// graph exit, ends run
    pub const END: Self = Self("__end__");
}

/// where to go after current node finishes and delta merged?
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Next {
    /// continue at other node
    Node(NodeId),

    /// stop (`NodeId::END`)
    End,
}

impl Next {
    /// normalize `NodeId::END` into `Next::End`
    #[must_use]
    pub fn from_node(id: NodeId) -> Self {
        if id == NodeId::END {
            Self::End
        } else {
            Self::Node(id)
        }
    }
}
