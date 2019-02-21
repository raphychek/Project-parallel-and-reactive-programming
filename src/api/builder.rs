//! API for creating graphs.
//!
//! This defines a variety of traits which can be used to provide a consistent API for creating
//! graphs.  This API requires manually finalizing nodes, but see the `ScopedGraphBuilder` in
//! `common::builder` for a simpler API.

use super::port::Port;
use std::ops::DerefMut;

/// A trait for types which can create graphs of nodes.
///
/// Runtime type should implement this trait by specifying the appropriate types.  The `GraphSpec`
/// functions are usually meant to be used in a single thread.
pub trait GraphSpec: Sized {
    /// The activator type used in control edges.  This is typically some sort of pointer into a
    /// trait object.
    type Activator;
}

/// A trait for types which can create new nodes.
///
/// We want `GraphSpec` instances to be able to create builders for all types of nodes, but with a
/// specific builder type which is able to handle the proper activator type.  Unfortunately, Rust
/// doesn't allow specifying type construtors as associated types in traits, so for instance the
/// following is invalid:
///
/// ```rust,ignore
/// trait GraphSpec {
///     type Builder<Node>;
///
///     fn node<Node>(&self, node: Node) -> Self::Builder<Node>;
/// }
/// ```
///
/// Instead, we use a `NodeSpec` trait which derives from `GraphSpec` and is parameterized by the
/// node type.  All types implementing `GraphSpec` should also implement `NodeSpec` for all node
/// types.
pub trait NodeSpec<Node>: GraphSpec {
    /// The builder type for nodes of type `Node`.
    type Builder: NodeBuilder<Self, Node = Node>;

    /// Create a new builder from a node.
    fn node(&self, node: Node) -> Self::Builder;
}

/// A type which can be used to create new ports.
///
/// Just like for `NodeSpec`, this should actually be a function of the `GraphSpec` trait, but Rust
/// doesn't allow us to express this.  Instead, all types implementing `GraphSpec` should usually
/// implement `PortSpec` for all supported data types with which users can create ports.
pub trait PortSpec<T> {
    /// The type of ports containing values of type `T`.
    type Port: Port;

    /// Create a new port with an initial value.
    fn port(&self, init: T) -> Self::Port;
}

/// A trait for types which can build nodes.
///
/// A builder represent a node which has been created, but was not fully initialized; typically,
/// this means a node which may have dangling input and/or output edges.
pub trait NodeBuilder<Spec: GraphSpec>: Sized {
    /// The underlying node type which will be built.
    type Node;

    /// Create a new activator for the underlying node.
    fn add_activator(&mut self) -> Spec::Activator;

    /// Finalize node creation.  This consumes the builder.
    ///
    /// Upon finalization, the builder should make sure the underlying node is ready to be
    /// scheduled.  This typically means initializing the pending count for the activators, and/or
    /// possibly scheduling the node immediately if there are no existing activators.
    fn finalize(&mut self, spec: &mut Spec);
}

/// A trait for borrowing the node from a builder.
///
/// We want `NodeBuilder` instances to provide a way to access the underlying `Node` instance to
/// update it, for instance to connect output activators.
pub trait NodeBorrowMut<'a, Spec: GraphSpec>: NodeBuilder<Spec> {
    type RefMut: DerefMut<Target = Self::Node> + 'a;

    fn borrow_mut(&'a mut self) -> Self::RefMut;
}
