//! ```text
//!   State₀  ──node A──▶  Δₐ
//!        ╲──node B──▶  Δᵦ     ──Merge──▶  State₁
//! ```
//! graph state, partial updates, and merging
//!
//! execution should proceed in supersteps or node transitions. at any moment the runtime holds one
//! complete `State`. nodes never mutate that snapshot as-is but rather emit a `StateDelta` which
//! describes their contribution to the state. the runtime folds these deltas into the next snapshot
//! using `Merge`. this is diagrammed as follows:
//!
//! State_0─┬──Node─A──►Delta_a─┐
//!         │                   │
//!         └──Node─B──►Delta_b─┴──►State_1
//!
//! keeping `State` and `StateDelta` as separate types allows a node to return only the fields it
//! touched. when a graph uses the same struct for both, implement [`Merge`] with `D = Self`.

/// complete graph at a single point in time:
///
/// this is the authoritative snapshot stored between supersteps and is passed to nodes as ro input.
/// concrete graphs ought to define a struct and implement this marker so bounds can be expressed
/// with a higher degree of generality (i.e. reducers)
pub trait State {}

/// partial update emitted by a single node:
///
/// a delta is intentionally incomplete as it represents what changed, not the full view. multiple
/// nodes in one superstep may each emit a delta; the runtime merges them into the next `State`. the
/// order in which this happens ought to be decided by the use-case.
///
/// like `State`, this is a marker for now. domain-specific graphs should add fields on their own
/// structs and implement `Merge` on the state type.
pub trait StateDelta {}

/// apply a partial update to accumulated state.
///
/// type parameter `D`. `D` is the delta type. it defaults to the implementing type when you write
/// `impl Merge for MyState`. when state and delta differ, write `impl Merge<MyDelta> for MyState`.
///
/// some ideas here:
///
/// when the type is the same for state and delta:
///
/// ```rust
/// use aperture::graph::state::Merge;
///
/// #[derive(Default, Clone)]
/// struct Counter {
///     n: u64,
/// }
///
/// impl Merge for Counter {
///     fn merge(&mut self, delta: Self) {
///         self.n += delta.n;
///     }
/// }
/// ```
///
/// when the delta type is separate:
///
/// ```rust
/// use aperture::graph::state::{Merge, State, StateDelta};
///
/// #[derive(Default)]
/// struct MyState {
///     messages: Vec<String>,
/// }
///
/// struct NewMessage {
///     text:String,
/// }
///
/// impl State for MyState {}
/// impl StateDelta for NewMessage {}
///
/// impl Merge<NewMessage> for MyState {
///     fn merge(&mut self, delta: NewMessage) {
///         self.messages.push(delta.text);
///     }
/// }
/// ```
pub trait Merge<D = Self> {
    /// fold `delta` into `self`, mutating the current state in place.
    ///
    /// merge should be associative enough for whatever the graph's reducer policy is, i.e. if three
    /// nodes emit delta_1, delta_2, and delta_3 in one superstep, `state.merge(delta_n)...` must
    /// match the intended semantics
    fn merge(&mut self, delta: D);
}
