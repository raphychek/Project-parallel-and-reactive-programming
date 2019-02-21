//! Activators should be used to implement the control structure of the graph.
//!
//! Conceptually, an activator contains a pointer or handle to a node, as well as an internal
//! counter indicating the number of times it must be activated before being scheduled (typically,
//! that counter would be initialized with the number of inputs to the node).  Activating then
//! means decrementing the counter and scheduling the node when the counter reaches zero.
//!
//! Note that the family of activator traits take a scheduler type as argument.  This allows to
//! have both generic activators compatible with multiple schedulers (this can allow code reuse
//! with a hybrid sequential/parallel scheduler for instance) and dynamic trait objects.
//!
//! In general, activators should be compatible with any scheduler which accept their underlying
//! handle type (see for instance the type bounds on the sequential activators).

/// A version of activation which takes a by-value activator.
///
/// Instances of `ActivatorOnce` can be activated, but it might not be possible to activate them
/// again.  This is the usual way nodes are activated in single-use dynamic graph.  Some
/// implementations (such as the sequential single use activator and the way it uses reference
/// counting) can be made more efficient by knowing the activator won't be reused.
pub trait ActivatorOnce<S> {
    /// Schedule the underlying handle if it is ready and consume the activator.
    fn activate_once(self, scheduler: &mut S);
}

/// A version of activation which takes a mutable activator.
///
/// Instances of `ActivatorMut` can be activated repeatedly and may mutate state.  This is the
/// usual way nodes are activated in multiple-use graphs.  The ability to mutate state in the
/// activator itself (and not the shared node handle) can be taken advantage of to build debug
/// wrappers which can log information about when the node is activated.
pub trait ActivatorMut<S>: ActivatorOnce<S> {
    /// When scheduling through this method, the activator is free to mutate any local state.
    ///
    /// After activation, the same`ActivatorMut` instance should not be activated again until the
    /// underlying node has started its next execution.
    fn activate_mut(&mut self, scheduler: &mut S);
}

/// A version of activation which takes an immutable activator.
///
/// Instances of `Activator` can be activated repeatedly without mutating local state.  This is not
/// a very useful trait, but it is included for the sake of consistency.  It can also be used to
/// implement nodes with a merge semantic where only one activation from among several source nodes
/// is required to schedule the node.
pub trait Activator<S>: ActivatorMut<S> {
    /// When scheduled through this method, the activator can not mutate any local state.
    ///
    /// After activation, the same `Activator` instance should not be activated again until the
    /// underlying node has started its next execution.
    fn activate(&self, scheduler: &mut S);
}
