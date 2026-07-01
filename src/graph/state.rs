//! graph state, partial updates, and merging
//!
//! execution should proceed in supersteps or node transitions. at any moment the runtime holds one
//! complete [`State`]. nodes never mutate that snapshot as-is but rather emit a [`StateDelta`] which
//! describes their contribution to the state. the runtime folds these deltas into the next snapshot
//! using [`Merge`]. this is diagrammed as follows:
//!
//! State_0─┬──Node─A──►Delta_a─┐
//!         │                   │
//!         └──Node─B──►Delta_b─┴──►State_1
//!
//! keeping [`State`] and [`StateDelta`] as separate types allows a node to return only the fields it
//! touched. when a graph uses the same struct for both, implement [`Merge`] with `D = Self`.

/// graph state object at a single point in time:
///
/// this is the authoritative blueprint associated with the unified maintained state.
/// state is stored between supersteps and is passed to nodes as input.
/// concrete graphs ought to define a struct and implement this marker so bounds can be expressed
/// with a higher degree of generality (i.e. reducers)
pub trait State {}

/// partial update on state emitted by a single node:
///
/// a delta is intentionally incomplete as it represents what changed in the state as a result of a
/// node operation.
/// a state delta is not the full view of a [`State`], it contains the [`State`] fields that a node will
/// modify, and those fields are comprised of only the changes made to that field.
/// nodes in one superstep may each emit a delta; the runtime will [`Merge`] them into the next [`State`].
/// the order in which this happens ought to be decided by the use-case.
///
/// ex:
/// State { name: "Christian" } -> Node(Add_Last_Name) -> StateDelta { name: "Farrell" } -> Append Reducer
/// -> State { name: "Christian Farrell" }
///
/// like [`State`], this is a marker for now. domain-specific graphs should add fields on their own
/// structs and implement [`Merge`] on the state type.
pub trait StateDelta {}

/// reducer trait to define behavior of how a [`StateDelta`] is merged into a accumulated [`State`].
///
/// type parameter `D` is the [`StateDelta`] type. it defaults to the implementing type when you write
/// `impl Merge for MyState` - meaning all deltas will use the same reducing logic for merging delta
/// into state. this is possible because it is possible to make any [`StateDelta`] from [`State`] as
/// [`StateDelta`] must be a subset of [`State`].
///
/// when state and delta differ, write `impl Merge<MyDelta> for MyState` - meaning you wish
/// implement some custom reducing logic for a specific MyDelta.
///
/// some ideas here:
///
/// implementing the default Merge type:
///
/// ```rust
/// use aperture::graph::state::Merge;
///
/// // Our [`State`] is a counter - it is simple and only contains a single field of n.
/// #[derive(Default, Clone)]
/// struct Counter {
///     n: u64,
/// }
///
/// // We implement our reducing logic such that we always add delta into self.
/// // This is the default behavior that will be used for merging Counter and all subsets of
/// // Counter that dont have custom Merge implementations.
/// // Default Merge MUST be implemented to be an acceptable Graph State. (TODO - Is this the case?)
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
/// // In this example - [`State`] contains a vector of String.
/// #[derive(Default)]
/// struct Chat {
///     messages: Vec<String>,
///     metadata: u32,
/// }
///
/// // [`StateDelta`] for a node that adds a new message to the chat
/// // notice how this type differs from Chat in its fields. It is not even a subset of Chat.
/// struct NewMessage {
///     text: String,
/// }
///
/// impl State for Chat {}
/// impl StateDelta for NewMessage {}
///
/// // default [`Merge`] implementation required to a valid Graph Node.
/// impl Merge for Chat {
///     fn merge(&mut self, delta: Self) {
///         self.messages.append(&mut delta.messages.clone());
///         self.metadata += delta.metadata;
///     }
/// }
///
/// // custom [`Merge`] implementation which defines how a NewMessage is reduced into the existing
/// // [`State`]: Chat. In this case we say NewMessage will append its text to messages.
/// impl Merge<NewMessage> for Chat {
///     fn merge(&mut self, delta: NewMessage) {
///         self.messages.push(delta.text);
///     }
/// }
/// ```
pub trait Merge<D = Self> {
    /// reduce `delta` into `self`, mutating the current state in place.
    ///
    /// merge should be associative enough for whatever the graph's reducer policy is, i.e. if three
    /// nodes emit delta_1, delta_2, and delta_3 in one superstep, `state.merge(delta_n)...` must
    /// match the intended semantics
    fn merge(&mut self, delta: D);
}
