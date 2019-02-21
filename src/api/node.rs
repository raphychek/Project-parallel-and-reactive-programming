//! The node API.
//!
//! A node represents an element of the graph which can be scheduled.  The `Node` family of traits
//! take as argument a scheduler `S` with which they are compatible.
//!
//! You are not expected to write `Node` implementations yourself; the main way to interact with
//! nodes is through the `TaskNode` implementation in the `common` module which bundles together a
//! task with some input and output edges.
//!
//! The node traits `NodeBox` and `NodeMut` should usually be used as trait objects for
//! encapsulating any kind of nodes compatible with a given scheduler.

/// A node which can be executed once and is consumed.
pub trait NodeOnce<S: ?Sized> {
    /// Executes and consumes the node
    fn execute_once(self, scheduler: &mut S);
}

/// A node which can be executed from a Box.
///
/// This allows calling boxed trait objects, which would otherwise require unsized unboxing.  The
/// reasons for this trait to exist mostly the same as to why the `FnBox` standard Rust trait
/// exists.
pub trait NodeBox<S: ?Sized> {
    /// Executes the node from a box
    fn execute_box(self: Box<Self>, scheduler: &mut S);
}

impl<S: ?Sized, N: NodeOnce<S>> NodeBox<S> for N {
    fn execute_box(self: Box<Self>, scheduler: &mut S) {
        (*self).execute_once(scheduler);
    }
}

/// A node which can be executed repeatedly and may mutate its state.
pub trait NodeMut<S: ?Sized> {
    fn execute_mut(&mut self, scheduler: &mut S);
}
