use std::{
    cell::RefCell,
    ops::DerefMut,
    rc::{Rc, Weak},
};

use api::builder::*;

pub trait GraphSpecExt: GraphSpec {
    /// Create a new scope for creating new nodes.
    ///
    /// This provides an API similar to the `scope` function in crossbeam or rayon.
    fn build_scope<'a, T>(
        &'a mut self,
        build_fn: impl FnOnce(&mut ScopedGraphBuilder<'a, Self>) -> T,
    ) -> T {
        build_fn(&mut ScopedGraphBuilder::new(self))
    }
}

impl<Spec: GraphSpec> GraphSpecExt for Spec {}

/// Wraps a node builder with a lifetime marker and automatically finalize the builder when
/// dropped.
///
/// The lifetime marker is used to enforce that no mutable reference to the source scheduler can be
/// taken while the builder is alive.  This helps prevent programmer errors where a node could end
/// up being activated before it was finalized, causing a panic due to wrong pending counts.
///
/// # Warning
///
/// The `ScopedNodeBuilder` doesn't add any safety guarantee to the underlying builder.  For
/// instance, the node could end up being executed in a different runtime than it was being built
/// for -- it is only meant to provide safeguards against some programmer errors.
pub struct ScopedNodeBuilder<'a, Spec: GraphSpec + 'a, B: NodeBuilder<Spec>> {
    spec: Weak<RefCell<&'a mut Spec>>,
    builder: B,
}

impl<'a, Spec: GraphSpec + 'a, NB: NodeBuilder<Spec>> ScopedNodeBuilder<'a, Spec, NB> {
    /// Create and return an activator for the underlying node.
    ///
    /// # Panics
    ///
    /// This may panic if the builder was already finalized.
    pub fn add_activator(&mut self) -> Spec::Activator {
        self.builder.add_activator()
    }

    /// Mutably borrows the wrapped node.
    ///
    /// The borrow lasts until the returned value is dropped.  The node cannot be borrowed again
    /// while this borrow is active.
    ///
    /// This is typically used to update a node's input and/or output edges when they were not
    /// initially known.
    ///
    /// # Warning
    ///
    /// Note that depending on the underlying implementation, it may be possible to create
    /// reference cycles by using this method.
    ///
    /// # Panics
    ///
    /// This may panic if the node is currently borrowed.
    pub fn borrow_mut<'b>(&'b mut self) -> impl DerefMut<Target = NB::Node> + 'b
    where
        NB: NodeBorrowMut<'b, Spec>,
    {
        self.builder.borrow_mut()
    }
}

/// Automatically finalize the node when the builder gets dropped.
impl<'a, Spec: GraphSpec + 'a, B: NodeBuilder<Spec>> Drop for ScopedNodeBuilder<'a, Spec, B> {
    fn drop(&mut self) {
        if let Some(spec) = self.spec.upgrade() {
            self.builder.finalize(&mut *spec.borrow_mut())
        } else {
            eprintln!("Scoped builder was dropped after its scope ended.");
        }
    }
}

/// Wraps a graph builder with a lifetime marker.
///
/// The lifetime marker is used to enforce that no mutable reference to the graph can exist while
/// the builder is alive, and that all created nodes are dropped before the graph can be used again
/// (see `ScopedNodeBuilder`).  This helps prevent programmer errors where a node could end up
/// being activated before it was finalized, causing a panic due to wrong pending counts.
pub struct ScopedGraphBuilder<'a, Spec: GraphSpec + 'a> {
    spec: Rc<RefCell<&'a mut Spec>>,
}

impl<'a, Spec: GraphSpec + 'a> ScopedGraphBuilder<'a, Spec> {
    fn new(spec: &'a mut Spec) -> Self {
        ScopedGraphBuilder {
            spec: Rc::new(RefCell::new(spec)),
        }
    }

    /// Create a new builder from a node.
    pub fn node<N: 'a>(&mut self, node: N) -> ScopedNodeBuilder<'a, Spec, Spec::Builder>
    where
        Spec: NodeSpec<N>,
    {
        ScopedNodeBuilder {
            builder: self.spec.borrow_mut().node(node),
            spec: Rc::downgrade(&self.spec),
        }
    }

    /// Create a new port with an initial value.
    pub fn port<T>(&self, init: T) -> Spec::Port
    where
        Spec: PortSpec<T>,
    {
        self.spec.borrow().port(init)
    }

    pub fn borrow_mut<'b, T>(&'b mut self) -> impl DerefMut<Target = &'a mut Spec> + 'b {
        self.spec.borrow_mut()
    }
}

/// Display an error message if there are remaining scoped node builders when the graph builder is
/// dropped.
impl<'a, Spec: GraphSpec + 'a> Drop for ScopedGraphBuilder<'a, Spec> {
    fn drop(&mut self) {
        if Rc::strong_count(&self.spec) != 1 {
            eprintln!("Some nodes were not finalized after scoped build.");
        }
    }
}
