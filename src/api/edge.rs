//! Edges combine the control structure of activators with the data flow structure of ports.
//!
//! In general, an edge will be in one of the following categories:
//!
//!  - Data dependency.  Those edge are the most common and they have both a data component and a
//!    control component.  They send data to a given node, then activate said node, activating it
//!    once the data for all its inputs has been sent.
//!  - Control dependency.  Those edge are similar to the data dependency edges, except that there
//!    is no data transfer -- they effectively are just an activator.  They can be used as regular
//!    edge (with `()` content) in a node implementation; however see the `control_output` field in
//!    the `TaskNode` implementation.
//!  - Pure data edge.  Those edges are only concerned with transferring data and will not activate
//!    any node.  Those are rarely used, but they can be used effectively in reusable graphs, for
//!    instance by serving as memory between multiple executions of a node.
//!
//! Like slots, we implement edges in two parts.  The `OutputEdge` family of traits correspond to
//! the `Sender` family of traits, but they take a `scheduler` in order to be able to activate an
//! `Activator`.  They represent a type which can be used to output values from an executing task.
//! The `InputEdge` family of traits correspond to the `Receiver` family of traits, but they also
//! take a `scheduler` as argument.  They represent a type which can be used to get the inputs of
//! an executing task.
//!
//! Note that the `InputEdge` interface could allow two-way control flow by notifying a producer
//! node that a value was read and activating generation of the following value, but this is
//! currently not implemented: in practice, the `InputEdge` traits are simply wrappers around the
//! `Receiver` traits.  We use the `InputEdge` traits not only for consistency and symmetry with
//! the `OutputEdge` traits, but also to allow writing debug properties which can access the
//! scheduler's data structures.

/// An output edge for a node.  Common trait encompassing both data and control components.
pub trait OutputEdgeOnce<S> {
    /// The data type that transits on the edge.
    type Item;

    /// Send data on the edge then activate potential users.
    fn send_activate_once(self, scheduler: &mut S, item: Self::Item);
}

/// An output edge which can be used from a box.
///
/// We want to be able to somehow use `Box<dyn OutputEdgeOnce<S, Item = I>>`, but we cannot call
/// the `send_activate_once` method from such an object because the underlying `dyn` pointer is
/// unsized and can't be moved out of the box.  Instead, much like the `FnBox` unstable trait for
/// closures, we implement an auxiliary `OutputEdgeBox` trait which provides a `send_activate_box`
/// method that can be used directly from a box.  The `OutputEdgeBox` trait is then automatically
/// implemented for all *sized* types implementing `OutputEdgeOnce`.
pub trait OutputEdgeBox<S>: OutputEdgeOnce<S> {
    fn send_activate_box(self: Box<Self>, scheduler: &mut S, item: Self::Item);
}

impl<S, E: OutputEdgeOnce<S>> OutputEdgeBox<S> for E {
    fn send_activate_box(self: Box<Self>, scheduler: &mut S, item: Self::Item) {
        (*self).send_activate_once(scheduler, item)
    }
}

/// An output edge which can be used repeatedly and may mutate local state.
pub trait OutputEdgeMut<S>: OutputEdgeBox<S> {
    fn send_activate_mut(&mut self, scheduler: &mut S, item: Self::Item);
}

/// An output edge which can be used repeatedly without mutating local state.
pub trait OutputEdge<S>: OutputEdgeMut<S> {
    fn send_activate(&self, scheduler: &mut S, item: Self::Item);
}

/// An input edge for a node which can only be used once.
pub trait InputEdgeOnce<S> {
    type Item;

    fn recv_activate_once(self, scheduler: &mut S) -> Self::Item;
}

/// An input edge which can be used from a box.  See the `OutputEdgeBox` trait for explanations.
pub trait InputEdgeBox<S>: InputEdgeOnce<S> {
    fn recv_activate_box(self: Box<Self>, scheduler: &mut S) -> Self::Item;
}

impl<S, E: InputEdgeOnce<S>> InputEdgeBox<S> for E {
    fn recv_activate_box(self: Box<Self>, scheduler: &mut S) -> Self::Item {
        (*self).recv_activate_once(scheduler)
    }
}

/// An input edge which can be used repeatedly and may mutate local state.
pub trait InputEdgeMut<S>: InputEdgeBox<S> {
    fn recv_activate_mut(&mut self, scheduler: &mut S) -> Self::Item;
}

/// An input edge which can be used repeatedly without mutating local state.
pub trait InputEdge<S>: InputEdgeMut<S> {
    fn recv_activate(&self, scheduler: &mut S) -> Self::Item;
}
